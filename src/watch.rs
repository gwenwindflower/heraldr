//! The watcher daemon and its lifecycle.
//!
//! One daemon per herdr session (keyed by socket path) holds an
//! events.subscribe stream for structural changes and polls the focused
//! pane's foreground process between events — the poll is what catches
//! transitions no event covers, like yazi handing the pane to nvim. Manifest
//! events all run `heraldr kick`, which spawns a watcher if none holds the
//! session's pidfile lock, so any activity resurrects a dead daemon.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, Write};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::json;

use crate::naming::{self, Icons};
use crate::reconcile::Pass;
use crate::rpc::Client;
use crate::snapshot::{Snapshot, foreground_program};
use crate::state::{Store, session_key, state_dir};

/// Everything that can change a label or a position. The subscription surface
/// is a superset of what plugin [[events]] deliver — pane.updated and
/// layout.updated exist only here.
// workspace.metadata_updated is deliberately absent: Heraldr's own $jump
// token reports would re-fire it every pass.
const SUBSCRIPTIONS: &[&str] = &[
    "workspace.created",
    "workspace.updated",
    "workspace.renamed",
    "workspace.moved",
    "workspace.closed",
    "workspace.focused",
    "worktree.created",
    "worktree.opened",
    "worktree.removed",
    "tab.created",
    "tab.closed",
    "tab.renamed",
    "tab.moved",
    "tab.focused",
    "pane.created",
    "pane.closed",
    "pane.updated",
    "pane.focused",
    "pane.moved",
    "pane.exited",
    "pane.agent_detected",
    "layout.updated",
];

/// How long after a full pass to keep a safety net: collapse toggles emit no
/// event at all, so a periodic pass is the only thing that ever sees them.
const SAFETY_NET: Duration = Duration::from_secs(5);

/// Burst coalescing: a structural change (a close, a move) emits several
/// events back-to-back, and our own renames echo as more. One settle delay
/// per burst turns them into one pass.
const SETTLE: Duration = Duration::from_millis(60);

enum Signal {
    Changed,
    Gone,
}

pub fn watch(poll: Duration) -> Result<()> {
    let client = Client::from_env();
    let Some(_lock) = PidLock::acquire(&client)? else {
        return Ok(()); // another watcher already owns this session
    };

    let reader = client.subscribe(SUBSCRIPTIONS)?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => {
                    let _ = tx.send(Signal::Gone);
                    return;
                }
                Ok(_) => {
                    if tx.send(Signal::Changed).is_err() {
                        return;
                    }
                }
            }
        }
    });

    full_pass(&client)?;
    let mut last_full = Instant::now();
    loop {
        match rx.recv_timeout(poll) {
            Ok(Signal::Gone) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Server gone or restarted; exit and let the next kick revive us.
                return Ok(());
            }
            Ok(Signal::Changed) => {
                std::thread::sleep(SETTLE);
                while let Ok(Signal::Changed) = rx.try_recv() {}
                if full_pass(&client).is_err() {
                    return Ok(());
                }
                last_full = Instant::now();
                // Drop the echoes of our own renames; a real event racing
                // this window is caught by the safety net.
                while let Ok(Signal::Changed) = rx.try_recv() {}
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if last_full.elapsed() >= SAFETY_NET {
                    if full_pass(&client).is_err() {
                        return Ok(());
                    }
                    last_full = Instant::now();
                } else if focused_pass(&client).is_err() {
                    return Ok(());
                }
            }
        }
    }
}

fn full_pass(client: &Client) -> Result<()> {
    // Reload the manifest and the store each pass: icons.conf edits apply
    // live, and one-shot commands (reset, clear) share the store on disk.
    let icons = Icons::load(&plugin_root().join("icons.conf"));
    let mut state = Store::load(client);
    Pass {
        client,
        icons: &icons,
        state: &mut state,
        clear: false,
        force_tab: None,
    }
    .run()
}

/// The between-events poll: name only the focused tab from its pane's live
/// foreground process. Two socket calls, no cross-tab work.
fn focused_pass(client: &Client) -> Result<()> {
    let snapshot = Snapshot::fetch(client)?;
    let (Some(pane_id), Some(tab_id)) = (&snapshot.focused_pane_id, &snapshot.focused_tab_id)
    else {
        return Ok(());
    };
    let Some(tab) = snapshot.tabs.iter().find(|t| &t.tab_id == tab_id) else {
        return Ok(());
    };
    let Some(program) = foreground_program(client, pane_id).ok().flatten() else {
        return Ok(());
    };
    let icons = Icons::load(&plugin_root().join("icons.conf"));
    let mut state = Store::load(client);
    let label = tab.label();
    let stripped = naming::strip_prefix(label);
    if !state.eligible(&tab.tab_id, stripped) {
        return state.save();
    }
    let name = naming::format(&program, &icons);
    let want = format!("{}{name}", naming::index_prefix(label).unwrap_or(""));
    if want != label
        && client
            .call("tab.rename", json!({ "tab_id": tab.tab_id, "label": want }))
            .is_err()
    {
        return Ok(());
    }
    state.record(&tab.tab_id, &name);
    state.save()
}

/// The plugin root, where icons.conf lives: HERDR_PLUGIN_ROOT when herdr
/// invoked us, or the current directory for direct development commands.
pub fn plugin_root() -> PathBuf {
    std::env::var_os("HERDR_PLUGIN_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Spawn a detached watcher unless one already holds the session's lock. The
/// child re-checks the lock itself, so two racing kicks resolve to one
/// daemon.
pub fn kick() -> Result<()> {
    let client = Client::from_env();
    if PidLock::holder(&client).is_some() {
        return Ok(());
    }
    let exe = std::env::current_exe().context("resolving heraldr binary path")?;
    std::process::Command::new(exe)
        .arg("watch")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawning heraldr watch")?;
    Ok(())
}

pub fn stop() -> Result<bool> {
    let client = Client::from_env();
    let Some(pid) = PidLock::holder(&client) else {
        return Ok(false);
    };
    unsafe { libc::kill(pid, libc::SIGTERM) };
    Ok(true)
}

/// One watcher per session, enforced with flock on a pidfile keyed by the
/// socket path. The lock dies with the process, so a crashed watcher never
/// wedges the slot; the pid inside is advisory (status, stop).
pub struct PidLock {
    _file: File,
}

impl PidLock {
    fn path(client: &Client) -> PathBuf {
        state_dir().join(format!("watch-{:016x}.pid", session_key(client)))
    }

    pub fn acquire(client: &Client) -> Result<Option<Self>> {
        let path = Self::path(client);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;
        let taken = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if taken != 0 {
            return Ok(None);
        }
        file.set_len(0)?;
        writeln!(file, "{}", std::process::id())?;
        Ok(Some(Self { _file: file }))
    }

    /// The pid of the live watcher for this session, if any.
    pub fn holder(client: &Client) -> Option<i32> {
        let path = Self::path(client);
        let file = File::open(&path).ok()?;
        let free = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } == 0;
        if free {
            return None; // nobody holds the exclusive lock
        }
        std::fs::read_to_string(&path).ok()?.trim().parse().ok()
    }
}
