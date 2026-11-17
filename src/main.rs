//! Heraldr — Herdr tab, workspace, and agent chrome.
//!
//! Tabs compose jump numbers, program icons, names, and optional agent titles.
//! Sidebar metadata exposes workspace jump numbers and repository context.
//! A per-session watcher follows events and foreground process transitions;
//! manual tab names remain under the user's control.

#![allow(
    clippy::struct_field_names,
    reason = "Snapshot fields match Herdr's wire names"
)]
#![allow(
    clippy::needless_pass_by_value,
    reason = "RPC calls accept owned JSON request values"
)]

mod agent_icons;
mod config;
mod labels;
mod naming;
mod reconcile;
mod rpc;
mod snapshot;
mod state;
mod titles;
mod watch;

use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde_json::Value;

use crate::config::Config;
use crate::reconcile::{Pass, Relabel};
use crate::rpc::Client;
use crate::snapshot::Snapshot;
use crate::state::Store;
use crate::watch::{PidLock, icon_path, load_icons};

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
kick` to keep the watcher alive. config.toml composes '$jump', '$icon', '$program', and \
'$session_title' in tab_label; session titles are opt-in. Supported agents use Herdr Agent \
Icons Max when detected, with agent_icons set to 'font' to force logos or 'none' for Nerd Font \
icons. Explicit user icons.conf rows win; compiled icons provide the fallback."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a commented default configuration without replacing user settings
    InitConfig,
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
    /// Adopt tabs into automatic naming now, manual names included
    Relabel {
        /// Which tabs to relabel; tab and workspace follow the invoking pane, else focus
        #[arg(long, value_enum, default_value_t = Scope::Tab)]
        scope: Scope,
    },
    /// Replace the session's watcher with one from this binary, keeping labels
    Restart,
    /// Remove jump numbers and sidebar tokens and stop the watcher
    Clear,
    /// Show watcher, socket, and state health
    Status,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::InitConfig => {
            let (path, created) = config::initialize()?;
            println!(
                "{} {}",
                if created { "Created" } else { "Keeping" },
                path.display()
            );
            Ok(())
        }
        Command::Watch { poll_ms } => watch::watch(Duration::from_millis(poll_ms)),
        Command::Kick => watch::kick(),
        Command::Reconcile => one_shot(false, &Relabel::Nothing),
        Command::Relabel { scope } => one_shot(false, &relabel_target(scope)?),
        Command::Restart => watch::restart(),
        Command::Clear => {
            // Stop the watcher first or it would immediately redress everything.
            watch::stop()?;
            one_shot(true, &Relabel::Nothing)
        }
        Command::Status => {
            status();
            Ok(())
        }
    }
}

fn one_shot(clear: bool, relabel: &Relabel) -> Result<()> {
    let client = Client::from_env();
    let icons = load_icons();
    let config = if clear {
        Config::default()
    } else {
        Config::load()?
    };
    let mut state = Store::load(&client);
    Pass {
        client: &client,
        icons: &icons,
        config: &config,
        state: &mut state,
        clear,
        relabel,
    }
    .run()
}

#[derive(Clone, Copy, ValueEnum)]
enum Scope {
    /// The invoking or focused tab
    Tab,
    /// Every tab in the invoking or focused workspace
    Workspace,
    /// Every tab in the session
    All,
}

/// Resolve a scope to concrete IDs: the invoking pane's context first, then
/// the plugin action context, then whatever Herdr has focused.
fn relabel_target(scope: Scope) -> Result<Relabel> {
    match scope {
        Scope::All => Ok(Relabel::All),
        Scope::Tab => {
            if let Some(tab) = caller_id("HERDR_TAB_ID", &["/tab/tab_id", "/tab/id", "/tab_id"]) {
                return Ok(Relabel::Tab(tab));
            }
            let snapshot = Snapshot::fetch(&Client::from_env())?;
            snapshot
                .focused_tab_id
                .map(Relabel::Tab)
                .context("no invoking or focused tab to relabel")
        }
        Scope::Workspace => {
            if let Some(workspace) = caller_id(
                "HERDR_WORKSPACE_ID",
                &["/workspace/workspace_id", "/workspace/id", "/workspace_id"],
            ) {
                return Ok(Relabel::Workspace(workspace));
            }
            let snapshot = Snapshot::fetch(&Client::from_env())?;
            snapshot
                .focused_tab_id
                .and_then(|tab| snapshot.tabs.into_iter().find(|t| t.tab_id == tab))
                .map(|tab| Relabel::Workspace(tab.workspace_id))
                .context("no invoking or focused workspace to relabel")
        }
    }
}

fn caller_id(variable: &str, context_pointers: &[&str]) -> Option<String> {
    if let Ok(id) = std::env::var(variable)
        && !id.is_empty()
    {
        return Some(id);
    }
    let context = std::env::var("HERDR_PLUGIN_CONTEXT_JSON").ok()?;
    let value = serde_json::from_str::<Value>(&context).ok()?;
    context_pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn status() {
    let client = Client::from_env();
    println!("config   {}", config::path().display());
    match Config::load() {
        Ok(config) => println!("settings {}", serde_json::json!(config)),
        Err(error) => println!("settings invalid: {error:#}"),
    }
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
    let icon_path = icon_path();
    let icons = load_icons();
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
}
