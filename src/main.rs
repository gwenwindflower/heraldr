//! Heraldr — Herdr tab, workspace, and agent chrome.
//!
//! Tabs auto-name as "[N] <icon> <program>" after their foreground program
//! (the shell at a bare prompt), workspaces and agents get the "[N] " prefix
//! of their 1-9 jump keybind, and manual renames always win. A per-session
//! watcher daemon drives everything over herdr's socket: pushed events for
//! structural changes, a focused-pane poll for foreground changes no event
//! covers. Ported from the pure-fish engine, itself a rework of
//! qu8n/herdr-automatic-rename.

mod naming;
mod reconcile;
mod rpc;
mod snapshot;
mod state;
mod watch;

use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::Value;

use crate::naming::Icons;
use crate::reconcile::Pass;
use crate::rpc::Client;
use crate::snapshot::Snapshot;
use crate::state::Store;
use crate::watch::{PidLock, plugin_root};

#[derive(Parser)]
#[command(
    name = "heraldr",
    version,
    about = "Herdr tab, workspace, and agent chrome: [N] <icon> <name> labels driven by the live session",
    long_about = "Heraldr keeps a Herdr session legible: tabs auto-name as '[N] <icon> <program>' after \
their live foreground program, and sidebar rows get composable display-only metadata tokens — \
'$jump' (a workspace's 1-9 jump number) on workspaces, '$space' and '$worktree' (repo name + \
nested worktree name) on panes for agent rows. Labels stay bare everywhere else (agents panel, \
navigator), and manual renames always win. A per-session watcher keeps everything current from \
herdr's event stream plus a focused-pane poll; herdr plugin events only need to run `heraldr \
kick` to keep the watcher alive. Icons reload from the hand-edited icons.conf manifest when \
available and otherwise use the copy compiled into the binary."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the watcher for this session (kick spawns this; rarely run by hand)
    Watch {
        /// Focused-pane poll interval in milliseconds
        #[arg(long, default_value_t = 400)]
        poll_ms: u64,
    },
    /// Ensure the session's watcher is running (the herdr event hook)
    Kick,
    /// Run one full reconcile pass now
    Reconcile,
    /// Re-adopt the current tab into automatic naming after a manual rename
    Reset,
    /// Strip all Heraldr chrome and stop the watcher (run before uninstalling)
    Clear,
    /// Show watcher, socket, and state health
    Status,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Watch { poll_ms } => watch::watch(Duration::from_millis(poll_ms)),
        Command::Kick => watch::kick(),
        Command::Reconcile => one_shot(false, None),
        Command::Reset => {
            let tab = reset_target()?;
            one_shot(false, tab.as_deref())
        }
        Command::Clear => {
            // Stop the watcher first or it would immediately redress everything.
            watch::stop()?;
            std::thread::sleep(Duration::from_millis(150));
            one_shot(true, None)
        }
        Command::Status => status(),
    }
}

fn one_shot(clear: bool, force_tab: Option<&str>) -> Result<()> {
    let client = Client::from_env();
    let icons = Icons::load(&plugin_root().join("icons.conf"));
    let mut state = Store::load(&client);
    Pass {
        client: &client,
        icons: &icons,
        state: &mut state,
        clear,
        force_tab,
    }
    .run()
}

/// The tab reset should re-adopt: the invoking pane's tab, the plugin action
/// context's tab, or the focused tab, in that order.
fn reset_target() -> Result<Option<String>> {
    if let Ok(tab) = std::env::var("HERDR_TAB_ID")
        && !tab.is_empty()
    {
        return Ok(Some(tab));
    }
    if let Ok(context) = std::env::var("HERDR_PLUGIN_CONTEXT_JSON")
        && let Ok(value) = serde_json::from_str::<Value>(&context)
    {
        let tab = value
            .pointer("/tab/tab_id")
            .or_else(|| value.pointer("/tab/id"))
            .or_else(|| value.pointer("/tab_id"))
            .and_then(Value::as_str);
        if let Some(tab) = tab {
            return Ok(Some(tab.to_string()));
        }
    }
    let client = Client::from_env();
    Ok(Snapshot::fetch(&client)?.focused_tab_id)
}

fn status() -> Result<()> {
    let client = Client::from_env();
    println!("socket   {}", client.socket_path().display());
    match client.call("ping", serde_json::json!({})) {
        Ok(pong) => {
            let version = pong.get("version").and_then(Value::as_str).unwrap_or("?");
            let protocol = pong.get("protocol").and_then(Value::as_i64).unwrap_or(0);
            println!("herdr    {version} (protocol {protocol})");
        }
        Err(err) => println!("herdr    unreachable: {err}"),
    }
    match PidLock::holder(&client) {
        Some(pid) => println!("watcher  running (pid {pid})"),
        None => println!("watcher  not running — any herdr event will kick one off"),
    }
    let icon_path = plugin_root().join("icons.conf");
    let icons = Icons::load(&icon_path);
    if icons.is_built_in() {
        println!(
            "icons    {} built-in rows ({} unavailable or incomplete)",
            icons.len(),
            icon_path.display()
        );
    } else {
        println!("icons    {} rows from {}", icons.len(), icon_path.display());
    }
    println!("state    {} tabs tracked", Store::load(&client).len());
    Ok(())
}
