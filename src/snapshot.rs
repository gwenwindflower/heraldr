//! Typed views over `session.snapshot` and `pane.process_info`.
//!
//! The snapshot returns workspaces, tabs, panes, and agents in the same array
//! order as the individual list commands, and the jump keybinds number by that
//! order — so preserving it end-to-end is load-bearing.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::rpc::Client;

#[derive(Debug, Default, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub focused_tab_id: Option<String>,
    #[serde(default)]
    pub focused_pane_id: Option<String>,
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub tabs: Vec<Tab>,
    #[serde(default)]
    pub panes: Vec<Pane>,
    #[serde(default)]
    pub agents: Vec<Agent>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Workspace {
    pub workspace_id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub worktree: Option<Worktree>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Worktree {
    #[serde(default)]
    pub repo_key: Option<String>,
    #[serde(default)]
    pub is_linked_worktree: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Tab {
    pub tab_id: String,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub pane_count: u32,
    #[serde(default)]
    pub focused: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Pane {
    pub pane_id: String,
    #[serde(default)]
    pub tab_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub focused: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Agent {
    #[serde(default)]
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub pane_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub agent_session: Option<AgentSession>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AgentSession {
    #[serde(default)]
    pub agent: Option<String>,
}

impl Workspace {
    pub fn label(&self) -> &str {
        self.label.as_deref().unwrap_or("")
    }

    pub fn repo_key(&self) -> Option<&str> {
        self.worktree
            .as_ref()
            .and_then(|w| w.repo_key.as_deref())
            .filter(|k| !k.is_empty())
    }

    pub fn is_linked(&self) -> bool {
        self.worktree.as_ref().is_some_and(|w| w.is_linked_worktree)
    }
}

impl Tab {
    pub fn label(&self) -> &str {
        self.label.as_deref().unwrap_or("")
    }
}

impl Agent {
    /// The rename target: agent commands address the hosting terminal.
    pub fn target(&self) -> Option<&str> {
        self.terminal_id.as_deref().or(self.pane_id.as_deref())
    }

    /// The display label the panel shows: the manual name, else detection.
    pub fn label(&self) -> &str {
        self.name.as_deref().or(self.agent.as_deref()).unwrap_or("")
    }

    /// The detected agent kind, used to recognize auto-named agents.
    pub fn detected(&self) -> &str {
        self.agent_session
            .as_ref()
            .and_then(|s| s.agent.as_deref())
            .or(self.agent.as_deref())
            .unwrap_or("")
    }
}

impl Snapshot {
    pub fn fetch(client: &Client) -> Result<Self> {
        let result = client.call("session.snapshot", json!({}))?;
        let snapshot = result
            .get("snapshot")
            .cloned()
            .context("session.snapshot result missing snapshot")?;
        Ok(serde_json::from_value(snapshot)?)
    }

    /// The pane a tab's name should be read from: the sole pane of a
    /// single-pane tab, else the globally focused pane when the tab is
    /// focused. A background multi-pane tab exposes no active pane and its
    /// name is left as-is until it is next focused.
    pub fn active_pane(&self, tab: &Tab) -> Option<&Pane> {
        let mut tab_panes = self
            .panes
            .iter()
            .filter(|p| p.tab_id.as_deref() == Some(tab.tab_id.as_str()));
        if tab.pane_count == 1 {
            return tab_panes.next();
        }
        if tab.focused {
            return self.panes.iter().find(|p| p.focused).or_else(|| {
                self.panes
                    .iter()
                    .find(|p| p.tab_id.as_deref() == Some(tab.tab_id.as_str()))
            });
        }
        None
    }
}

/// The foreground program of a pane: the process-group leader's argv0
/// basename (at a bare prompt the leader is the shell itself), with a login
/// shell's leading "-" removed. argv0 is preferred over .name, which agents
/// like claude fill with a version string. `None` means the sample failed or
/// resolved nothing — callers keep the current label rather than guess.
pub fn foreground_program(client: &Client, pane_id: &str) -> Result<Option<String>> {
    let result = client.call("pane.process_info", json!({ "pane_id": pane_id }))?;
    let info = result
        .get("process_info")
        .context("pane.process_info result missing process_info")?;
    let Some(group) = info
        .get("foreground_process_group_id")
        .and_then(Value::as_i64)
    else {
        return Ok(None);
    };
    let Some(procs) = info.get("foreground_processes").and_then(Value::as_array) else {
        return Ok(None);
    };
    let leader = procs
        .iter()
        .find(|p| p.get("pid").and_then(Value::as_i64) == Some(group));
    Ok(leader.and_then(|p| {
        let argv0 = p
            .get("argv0")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .or_else(|| p.get("name").and_then(Value::as_str))?;
        let base = argv0.strip_prefix('-').unwrap_or(argv0);
        let base = base.rsplit('/').next().unwrap_or("");
        (!base.is_empty()).then(|| base.to_string())
    }))
}
