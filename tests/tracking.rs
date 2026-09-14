use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

#[derive(Default)]
struct Session {
    tab_id: String,
    label: String,
    program: String,
    inspection_fails: bool,
    leader_missing: bool,
    rename_fails: bool,
    inspections: usize,
    renames: usize,
}

struct Herdr {
    root: PathBuf,
    state_root: PathBuf,
    session: Arc<Mutex<Session>>,
    stopped: Arc<AtomicBool>,
    server: Option<JoinHandle<()>>,
    watcher: Option<Child>,
}

impl Herdr {
    fn start() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "heraldr-tracking-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let listener = UnixListener::bind(root.join("herdr.sock")).unwrap();
        listener.set_nonblocking(true).unwrap();
        let session = Arc::new(Mutex::new(Session {
            tab_id: "w1:t1".into(),
            label: "1".into(),
            program: "lazygit".into(),
            ..Session::default()
        }));
        let stopped = Arc::new(AtomicBool::new(false));
        let peer_session = Arc::clone(&session);
        let peer_stopped = Arc::clone(&stopped);
        let server = thread::spawn(move || {
            let mut subscriptions = Vec::new();
            while !peer_stopped.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(err) => panic!("accepting test connection: {err}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut line = String::new();
                BufReader::new(&stream).read_line(&mut line).unwrap();
                let request: Value = serde_json::from_str(&line).unwrap();
                let method = request["method"].as_str().unwrap();
                let mut session = peer_session.lock().unwrap();
                let result = match method {
                    "events.subscribe" => json!({}),
                    "session.snapshot" => json!({ "snapshot": {
                        "focused_tab_id": session.tab_id, "focused_pane_id": "w1:p1",
                        "workspaces": [{ "workspace_id": "w1", "label": "project" }],
                        "tabs": [{ "workspace_id": "w1", "tab_id": session.tab_id,
                            "label": session.label, "pane_count": 1, "focused": true }],
                        "panes": [{ "workspace_id": "w1", "tab_id": session.tab_id,
                            "pane_id": "w1:p1", "focused": true }]
                    }}),
                    "pane.process_info" => {
                        session.inspections += 1;
                        let processes = if session.leader_missing {
                            json!([])
                        } else {
                            json!([{ "pid": 42, "argv0": session.program }])
                        };
                        json!({ "process_info": {
                            "foreground_process_group_id": 42,
                            "foreground_processes": processes
                        }})
                    }
                    "tab.rename" => {
                        session.renames += 1;
                        if !session.rename_fails {
                            session.label = request["params"]["label"].as_str().unwrap().into();
                        }
                        json!({ "tab": { "tab_id": session.tab_id, "label": session.label } })
                    }
                    "workspace.report_metadata" | "pane.report_metadata" => json!({}),
                    other => panic!("unexpected method: {other}"),
                };
                let failed = (method == "pane.process_info" && session.inspection_fails)
                    || (method == "tab.rename" && session.rename_fails);
                let response = if failed {
                    json!({ "id": request["id"], "error": { "message": "temporarily unavailable" } })
                } else {
                    json!({ "id": request["id"], "result": result })
                };
                writeln!(stream, "{response}").unwrap();
                if method == "events.subscribe" {
                    subscriptions.push(stream);
                }
            }
        });
        Self {
            state_root: root.join("state"),
            root,
            session,
            stopped,
            server: Some(server),
            watcher: None,
        }
    }

    fn command(&self, subcommand: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_heraldr"));
        command
            .arg(subcommand)
            .env("HERDR_SOCKET_PATH", self.root.join("herdr.sock"))
            .env("HERALDR_STATE_DIR", &self.state_root)
            .env("HERDR_PLUGIN_ROOT", env!("CARGO_MANIFEST_DIR"));
        command
    }

    fn reconcile(&self) {
        let output = self.command("reconcile").output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn reset(&self) {
        let tab_id = self.session.lock().unwrap().tab_id.clone();
        let output = self
            .command("reset")
            .env("HERDR_TAB_ID", tab_id)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn watch(&mut self) {
        self.watcher = Some(
            self.command("watch")
                .args(["--poll-ms", "10"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        self.wait_for(|session| session.label.ends_with(" lazygit"));
    }

    fn change(&self, update: impl FnOnce(&mut Session)) {
        update(&mut self.session.lock().unwrap());
    }

    fn label(&self) -> String {
        self.session.lock().unwrap().label.clone()
    }

    fn wait_for(&mut self, condition: impl Fn(&Session) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(watcher) = &mut self.watcher {
                assert!(
                    watcher.try_wait().unwrap().is_none(),
                    "watcher exited during process transition"
                );
            }
            if condition(&self.session.lock().unwrap()) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "tracking did not recover; label: {}",
                self.label()
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn follows(&mut self, program: &str) {
        self.change(|session| session.program = program.into());
        self.wait_for(|session| session.label.ends_with(&format!(" {program}")));
    }
}

impl Drop for Herdr {
    fn drop(&mut self) {
        if let Some(watcher) = &mut self.watcher {
            let _ = watcher.kill();
            let _ = watcher.wait();
        }
        self.stopped.store(true, Ordering::Relaxed);
        let result = self.server.take().unwrap().join();
        let _ = std::fs::remove_dir_all(&self.root);
        if !thread::panicking() {
            result.unwrap();
        }
    }
}

#[test]
fn failed_full_pass_rename_retains_ownership_and_recovers() {
    let herdr = Herdr::start();
    herdr.reconcile();
    let lazygit = herdr.label();
    herdr.change(|session| {
        session.program = "1Password".into();
        session.rename_fails = true;
    });
    herdr.reconcile();
    assert_eq!(herdr.label(), lazygit);
    herdr.change(|session| session.rename_fails = false);
    herdr.reconcile();
    assert!(herdr.label().ends_with(" fish"));
    for program in ["lazygit", "fish", "nvim"] {
        herdr.change(|session| session.program = program.into());
        herdr.reconcile();
        assert!(herdr.label().ends_with(&format!(" {program}")));
    }
}

#[test]
fn focused_poll_survives_process_inspection_failure() {
    let mut herdr = Herdr::start();
    herdr.watch();
    let lazygit = herdr.label();
    herdr.change(|session| {
        session.inspection_fails = true;
        session.program = "1Password".into();
        session.inspections = 0;
    });
    herdr.wait_for(|session| session.inspections >= 3);
    assert_eq!(herdr.label(), lazygit);
    herdr.change(|session| session.inspection_fails = false);
    herdr.follows("lazygit");
    herdr.follows("fish");
    herdr.follows("nvim");
}

#[test]
fn focused_poll_survives_rename_failure() {
    let mut herdr = Herdr::start();
    herdr.watch();
    let lazygit = herdr.label();
    herdr.change(|session| {
        session.rename_fails = true;
        session.program = "sudo".into();
        session.renames = 0;
    });
    herdr.wait_for(|session| session.renames >= 3);
    assert_eq!(herdr.label(), lazygit);
    herdr.change(|session| session.rename_fails = false);
    herdr.follows("nvim");
    herdr.follows("fish");
}

#[test]
fn missing_foreground_leader_preserves_label_and_recovers() {
    let mut herdr = Herdr::start();
    herdr.watch();
    let lazygit = herdr.label();
    herdr.change(|session| {
        session.leader_missing = true;
        session.inspections = 0;
    });
    herdr.wait_for(|session| session.inspections >= 3);
    assert_eq!(herdr.label(), lazygit);
    herdr.change(|session| session.leader_missing = false);
    herdr.follows("fish");
    herdr.follows("nvim");
}

#[test]
fn manual_name_survives_process_transitions() {
    let herdr = Herdr::start();
    herdr.reconcile();
    herdr.change(|session| session.label = "[1] release work".into());
    for program in ["1Password", "lazygit", "fish", "nvim"] {
        herdr.change(|session| session.program = program.into());
        herdr.reconcile();
        assert_eq!(herdr.label(), "[1] release work");
    }
}

#[test]
fn sessions_with_matching_tab_ids_keep_independent_ownership() {
    let first = Herdr::start();
    let mut second = Herdr::start();
    second.state_root = first.state_root.clone();
    first.reconcile();
    second.change(|session| session.program = "nvim".into());
    second.reconcile();
    assert!(second.label().ends_with(" nvim"));
    assert!(first.label().ends_with(" lazygit"));

    second.change(|session| session.label = "[1] release work".into());
    second.reconcile();
    first.change(|session| session.program = "fish".into());
    first.reconcile();
    assert!(first.label().ends_with(" fish"));
    second.reconcile();
    assert_eq!(second.label(), "[1] release work");
}

#[test]
fn shared_ownership_is_preserved_without_cross_session_writes() {
    let first = Herdr::start();
    let mut second = Herdr::start();
    second.state_root = first.state_root.clone();
    std::fs::create_dir_all(&first.state_root).unwrap();
    let shared = first.state_root.join("state.json");
    let label = "[1] G lazygit";
    let record = json!({"w1:t1": {"auto": "G lazygit", "enabled": true}}).to_string();
    std::fs::write(&shared, &record).unwrap();
    first.change(|session| {
        session.label = label.into();
        session.program = "fish".into();
    });
    first.reconcile();
    assert!(first.label().ends_with(" fish"));
    assert_eq!(std::fs::read_to_string(&shared).unwrap(), record);

    second.change(|session| {
        session.label = label.into();
        session.program = "nvim".into();
    });
    second.reconcile();
    assert!(second.label().ends_with(" nvim"));
    first.change(|session| session.program = "yazi".into());
    first.reconcile();
    assert!(first.label().ends_with(" yazi"));
    assert_eq!(std::fs::read_to_string(&shared).unwrap(), record);
}

#[test]
fn another_session_cannot_prune_a_running_tabs_ownership() {
    let first = Herdr::start();
    let mut second = Herdr::start();
    second.state_root = first.state_root.clone();
    first.change(|session| session.tab_id = "w1X:t9".into());
    first.reconcile();
    assert!(first.label().ends_with(" lazygit"));

    second.reconcile();
    for program in ["1Password", "lazygit", "fish", "yazi"] {
        first.change(|session| session.program = program.into());
        first.reconcile();
        let expected = if program == "1Password" {
            "fish"
        } else {
            program
        };
        assert!(first.label().ends_with(&format!(" {expected}")));
        second.reconcile();
    }
}

#[test]
fn simultaneous_watchers_follow_their_own_foreground_programs() {
    let mut first = Herdr::start();
    let mut second = Herdr::start();
    second.state_root = first.state_root.clone();
    first.watch();
    second.watch();
    first.follows("fish");
    second.follows("nvim");
    first.follows("yazi");
    second.follows("fish");
    first.follows("lazygit");
}

#[test]
fn shared_manual_opt_out_survives_session_initialization() {
    let herdr = Herdr::start();
    std::fs::create_dir_all(&herdr.state_root).unwrap();
    let shared = herdr.state_root.join("state.json");
    let record = json!({"w1:t1": {"auto": "", "enabled": false}}).to_string();
    std::fs::write(&shared, &record).unwrap();
    herdr.change(|session| session.label = "[1] release work".into());
    herdr.reconcile();
    herdr.change(|session| session.program = "yazi".into());
    herdr.reconcile();
    assert_eq!(herdr.label(), "[1] release work");
    assert_eq!(std::fs::read_to_string(shared).unwrap(), record);
}

#[test]
fn reset_recovers_a_disabled_tab_while_another_session_reconciles() {
    let first = Herdr::start();
    let mut second = Herdr::start();
    second.state_root = first.state_root.clone();
    std::fs::create_dir_all(&first.state_root).unwrap();
    std::fs::write(
        first.state_root.join("state.json"),
        json!({"w1X:t9": {"auto": "", "enabled": false}}).to_string(),
    )
    .unwrap();
    first.change(|session| {
        session.tab_id = "w1X:t9".into();
        session.label = "[1] G lazygit".into();
        session.program = "fish".into();
    });
    first.reconcile();
    assert_eq!(first.label(), "[1] G lazygit");
    second.reconcile();
    first.reset();
    assert!(first.label().ends_with(" fish"));
    second.reconcile();
    first.change(|session| session.program = "yazi".into());
    first.reconcile();
    assert!(first.label().ends_with(" yazi"));
}
