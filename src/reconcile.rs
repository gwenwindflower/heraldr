//! The unified reconcile: one pass computes every workspace, tab, and agent
//! label and issues one rename per item whose label is wrong. Every rename is
//! skip-if-correct, so re-running the pass (herdr re-emits rename events for
//! our own renames) changes nothing and cannot loop.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use serde_json::{Value, json};

use crate::naming::{self, Icons};
use crate::rpc::Client;
use crate::snapshot::{Snapshot, Workspace, foreground_program};
use crate::state::Store;

const METADATA_SOURCE: &str = "heraldr";
const JUMP_TOKEN: &str = "jump";
const JUMP_TTL_MS: u64 = 30_000;
const SPACE_TOKEN: &str = "space";
const WORKTREE_TOKEN: &str = "worktree";

pub struct Pass<'a> {
    pub client: &'a Client,
    pub icons: &'a Icons,
    pub state: &'a mut Store,
    pub clear: bool,
    pub force_tab: Option<&'a str>,
}

impl Pass<'_> {
    pub fn run(&mut self) -> Result<()> {
        let snapshot = Snapshot::fetch(self.client)?;
        let session_dir = self.client.session_dir();
        if let Some(tab) = self.force_tab {
            self.state.forget(tab);
        }
        self.workspaces(&snapshot, &session_dir);
        self.tabs(&snapshot);
        self.panes(&snapshot);
        self.agents(&snapshot);
        self.state.save()
    }

    /// Workspace labels stay bare; the jump number rides a display-only
    /// metadata token instead, so only sidebar rows that opt into "$jump"
    /// show it — the agents panel and the navigator keep the plain name.
    /// Tokens carry a TTL refreshed by the watcher's safety-net pass, so a
    /// dead watcher's numbers fade out instead of sticking stale.
    fn workspaces(&self, snapshot: &Snapshot, session_dir: &Path) {
        let collapsed = collapsed_spaces(session_dir);
        for (index, position) in workspace_positions(&snapshot.workspaces, &collapsed) {
            let workspace = &snapshot.workspaces[index];
            let label = workspace.label();
            let base = naming::strip_prefix(label);
            if !base.is_empty() && base != label {
                // Heal a "[N] " prefix a label-renaming Heraldr left behind.
                let _ = self.client.call(
                    "workspace.rename",
                    json!({ "workspace_id": workspace.workspace_id, "label": base }),
                );
            }
            let jump = if !self.clear && (1..=9).contains(&position) {
                Value::from(format!("[{position}]"))
            } else {
                Value::Null // explicit clear; hidden and 10+ rows show nothing
            };
            let _ = self.client.call(
                "workspace.report_metadata",
                json!({
                    "workspace_id": workspace.workspace_id,
                    "source": METADATA_SOURCE,
                    "tokens": { JUMP_TOKEN: jump },
                    "ttl_ms": JUMP_TTL_MS,
                }),
            );
        }
    }

    /// Tabs are the surface where naming and numbering meet: per tab the base
    /// is computed once (fresh name if owned/eligible, else the stripped
    /// current base) and the position prefix lands in the same single rename,
    /// so a brand-new tab settles at "[3] <icon> fish" with no flicker.
    fn tabs(&mut self, snapshot: &Snapshot) {
        let mut seen = HashSet::new();
        for workspace in &snapshot.workspaces {
            let mut position = 0;
            for tab in snapshot
                .tabs
                .iter()
                .filter(|t| t.workspace_id == workspace.workspace_id)
            {
                position += 1;
                seen.insert(tab.tab_id.clone());
                let label = tab.label();
                let stripped = naming::strip_prefix(label).to_string();
                let mut base = stripped.clone();
                let mut named = false;
                if !self.clear {
                    let forced = self.force_tab == Some(tab.tab_id.as_str());
                    if forced || self.state.eligible(&tab.tab_id, &stripped) {
                        let program = snapshot
                            .active_pane(tab)
                            .and_then(|p| foreground_program(self.client, &p.pane_id).ok())
                            .flatten();
                        if let Some(program) = program {
                            base = naming::format(&program, self.icons);
                            named = true;
                            self.state.record(&tab.tab_id, &base);
                        }
                    }
                }
                // No sensible "[N] " exists for an empty base, and an
                // un-named all-digit base is herdr's transient placeholder:
                // numbering it now would flash a throwaway "[3] 3". Defer
                // both, but keep counting so later tabs stay correct.
                if base.is_empty() {
                    continue;
                }
                if !self.clear && !named && naming::is_placeholder(&base) {
                    continue;
                }
                let want = naming::desired(position, &base, self.clear);
                if want != label {
                    let _ = self
                        .client
                        .call("tab.rename", json!({ "tab_id": tab.tab_id, "label": want }));
                }
            }
        }
        if !self.clear {
            self.state.prune(&seen);
        }
    }

    /// Agent sidebar rows read pane metadata, so every pane carries its
    /// workspace's display pair: $space is the repo's main-checkout label,
    /// and $worktree is this workspace's own label when it rides under that
    /// parent as a linked worktree. Rows compose them as `space worktree`,
    /// keeping the flat agents panel meaningful when a branch name says
    /// nothing about its repo. No TTL: workspace names are stable, losing
    /// them on watcher death would hurt more than a briefly stale pair, and
    /// pane/workspace events refresh them live.
    fn panes(&self, snapshot: &Snapshot) {
        let names = space_names(&snapshot.workspaces);
        for pane in &snapshot.panes {
            let pair = pane
                .workspace_id
                .as_deref()
                .and_then(|id| names.get(id).copied());
            let (space, worktree) = match (self.clear, pair) {
                (false, Some((space, worktree))) => (
                    Value::from(space),
                    worktree.map(Value::from).unwrap_or(Value::Null),
                ),
                _ => (Value::Null, Value::Null),
            };
            let _ = self.client.call(
                "pane.report_metadata",
                json!({
                    "pane_id": pane.pane_id,
                    "source": METADATA_SOURCE,
                    "tokens": { SPACE_TOKEN: space, WORKTREE_TOKEN: worktree },
                }),
            );
        }
    }

    /// Heraldr leaves agents alone — the agent panel keeps its own order and
    /// its own jump keybinds, so tab/workspace-style chrome is pure noise
    /// there. This pass only heals leftovers from a label-renaming Heraldr: a
    /// "[N] " prefix or a stuck park temp reverts to the bare name.
    fn agents(&self, snapshot: &Snapshot) {
        for agent in &snapshot.agents {
            let Some(target) = agent.target() else {
                continue;
            };
            let label = agent.label();
            let detected = agent.detected();
            let base = unpark(naming::strip_prefix(label), detected);
            if base != label {
                self.revert_agent(target, base, detected);
            }
        }
    }

    /// Reverting an auto-named agent clears back to detection (which also
    /// sidesteps duplicate-name rejection when several agents share a base
    /// like "claude"); a user-named agent keeps its name.
    fn revert_agent(&self, target: &str, base: &str, detected: &str) {
        let name = if !detected.is_empty() && base == detected {
            Value::Null
        } else {
            Value::from(base)
        };
        let _ = self
            .client
            .call("agent.rename", json!({ "target": target, "name": name }));
    }
}

/// Space keys whose sidebar group is collapsed right now. herdr exposes
/// collapse nowhere in its API; session.json's top-level collapsed_space_keys
/// (written on a 5s debounce) is the one readable copy, so numbers can lag a
/// collapse until the next event or safety-net pass.
pub fn collapsed_spaces(session_dir: &Path) -> HashSet<String> {
    std::fs::read_to_string(session_dir.join("session.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| {
            v.get("collapsed_space_keys")
                .and_then(Value::as_array)
                .map(|keys| {
                    keys.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
        })
        .unwrap_or_default()
}

/// Each workspace's agent-row display pair, keyed by workspace_id: the
/// space's main-checkout label, plus the workspace's own label when it is a
/// linked worktree nested under that main checkout. The nesting rule matches
/// workspace_positions (a repo groups only with 2+ open workspaces and a
/// main checkout), so the agents panel and the spaces panel always agree on
/// what counts as a worktree.
pub fn space_names(workspaces: &[Workspace]) -> HashMap<&str, (&str, Option<&str>)> {
    let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in workspaces.iter().enumerate() {
        if let Some(key) = w.repo_key() {
            groups.entry(key).or_default().push(i);
        }
    }
    let mut names: HashMap<&str, (&str, Option<&str>)> = workspaces
        .iter()
        .map(|w| (w.workspace_id.as_str(), (w.label(), None)))
        .collect();
    for members in groups.values() {
        let Some(&main) = members.iter().find(|&&i| !workspaces[i].is_linked()) else {
            continue;
        };
        if members.len() < 2 {
            continue;
        }
        for &i in members {
            if i != main && workspaces[i].is_linked() {
                names.insert(
                    workspaces[i].workspace_id.as_str(),
                    (workspaces[main].label(), Some(workspaces[i].label())),
                );
            }
        }
    }
    names
}

/// Each workspace's 1-based slot in herdr's VISIBLE sidebar order (0 = the
/// sidebar does not render it). The jump keybind resolves through that
/// visible order, not the raw list order, so this mirrors herdr's own
/// workspace_list_entries_inner:
///   * Workspaces sharing a worktree repo_key nest into one "space", but only
///     when the repo has 2+ open workspaces and one is the main checkout.
///   * A space renders at the slot of its first-appearing member, headed by
///     the main checkout, with the other members nested after it in array
///     order.
///   * A collapsed space renders its head row alone — except the focused
///     member, which herdr keeps rendered under its parent.
pub fn workspace_positions(
    workspaces: &[Workspace],
    collapsed: &HashSet<String>,
) -> Vec<(usize, usize)> {
    let mut spaces: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, w) in workspaces.iter().enumerate() {
        if let Some(key) = w.repo_key() {
            spaces.entry(key).or_default().push(i);
        }
    }
    spaces.retain(|_, members| {
        let Some(head) = members.iter().position(|&i| !workspaces[i].is_linked()) else {
            return false;
        };
        if members.len() < 2 {
            return false;
        }
        let head = members.remove(head);
        members.insert(0, head);
        true
    });

    let active = workspaces.iter().position(|w| w.focused);
    let mut order: Vec<usize> = Vec::new();
    for (i, w) in workspaces.iter().enumerate() {
        let Some(members) = w.repo_key().and_then(|k| spaces.get(k)) else {
            order.push(i);
            continue;
        };
        // The space renders once, at the slot of its first-appearing member.
        if i != *members.iter().min().expect("space has members") {
            continue;
        }
        order.push(members[0]);
        let key = w.repo_key().expect("grouped workspace has a key");
        if collapsed.contains(key) {
            if let Some(a) = active
                && a != members[0]
                && workspaces[a].repo_key() == Some(key)
            {
                order.push(a);
            }
        } else {
            order.extend(members[1..].iter().copied());
        }
    }

    workspaces
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let position = order.iter().position(|&o| o == i).map_or(0, |p| p + 1);
            (i, position)
        })
        .collect()
}

/// A base with a stuck park-temp suffix removed. If a finalize ever loses to
/// herdr, the agent stays at "[N] <base> <target>"; the glued target would
/// freeze the agent, so a trailing park token is dropped ONLY when what
/// remains is exactly the detected kind — a real multi-word user name is
/// untouched.
pub fn unpark<'a>(base: &'a str, detected: &str) -> &'a str {
    if detected.is_empty() {
        return base;
    }
    let Some(rest) = base.strip_prefix(detected) else {
        return base;
    };
    let Some(token) = rest.strip_prefix(' ') else {
        return base;
    };
    let is_park_token = token.starts_with("term_")
        || (token.starts_with('w')
            && token[1..].starts_with(|c: char| c.is_ascii_digit())
            && token.contains(':'));
    if is_park_token && !token.contains(' ') {
        detected_prefix(base, detected)
    } else {
        base
    }
}

fn detected_prefix<'a>(base: &'a str, detected: &str) -> &'a str {
    &base[..detected.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(id: &str, label: &str, focused: bool, worktree: Option<(&str, bool)>) -> Workspace {
        Workspace {
            workspace_id: id.to_string(),
            label: Some(label.to_string()),
            focused,
            worktree: worktree.map(|(key, linked)| crate::snapshot::Worktree {
                repo_key: Some(key.to_string()),
                is_linked_worktree: linked,
            }),
        }
    }

    fn positions(workspaces: &[Workspace], collapsed: &[&str]) -> Vec<usize> {
        let collapsed = collapsed.iter().map(|s| s.to_string()).collect();
        workspace_positions(workspaces, &collapsed)
            .into_iter()
            .map(|(_, p)| p)
            .collect()
    }

    #[test]
    fn plain_workspaces_number_in_array_order() {
        let all = [
            ws("w1", "a", true, None),
            ws("w2", "b", false, None),
            ws("w3", "c", false, None),
        ];
        assert_eq!(positions(&all, &[]), vec![1, 2, 3]);
    }

    #[test]
    fn worktree_space_nests_under_its_main_checkout() {
        // The linked worktree appears first in array order, but the space
        // renders at that first slot headed by the main checkout.
        let all = [
            ws("w1", "feat", false, Some(("repo", true))),
            ws("w2", "other", true, None),
            ws("w3", "main", false, Some(("repo", false))),
        ];
        assert_eq!(positions(&all, &[]), vec![2, 3, 1]);
    }

    #[test]
    fn lone_or_headless_groups_stay_flat() {
        let all = [
            ws("w1", "solo", false, Some(("repo-a", false))),
            ws("w2", "linked-1", true, Some(("repo-b", true))),
            ws("w3", "linked-2", false, Some(("repo-b", true))),
        ];
        assert_eq!(positions(&all, &[]), vec![1, 2, 3]);
    }

    #[test]
    fn collapse_hides_members_but_keeps_the_focused_one() {
        let all = [
            ws("w1", "main", false, Some(("repo", false))),
            ws("w2", "feat", true, Some(("repo", true))),
            ws("w3", "other", false, None),
        ];
        assert_eq!(positions(&all, &[]), vec![1, 2, 3]);
        assert_eq!(
            positions(&all, &["repo"]),
            vec![1, 2, 3],
            "focused member stays rendered"
        );
        let unfocused = [
            ws("w1", "main", false, Some(("repo", false))),
            ws("w2", "feat", false, Some(("repo", true))),
            ws("w3", "other", true, None),
        ];
        assert_eq!(
            positions(&unfocused, &["repo"]),
            vec![1, 0, 2],
            "hidden member reports 0"
        );
    }

    #[test]
    fn space_names_compose_parent_and_worktree() {
        let all = [
            ws("w1", "cloudy", false, Some(("repo", false))),
            ws("w2", "feat-docs-agent", false, Some(("repo", true))),
            ws("w3", "solo", true, None),
            ws("w4", "lonely-linked", false, Some(("other", true))),
        ];
        let names = space_names(&all);
        assert_eq!(names["w1"], ("cloudy", None));
        assert_eq!(names["w2"], ("cloudy", Some("feat-docs-agent")));
        assert_eq!(names["w3"], ("solo", None));
        assert_eq!(
            names["w4"],
            ("lonely-linked", None),
            "a linked worktree with no open main checkout stays flat"
        );
    }

    #[test]
    fn unpark_recovers_only_stuck_park_temps() {
        assert_eq!(unpark("claude term_ab12", "claude"), "claude");
        assert_eq!(unpark("claude w1:p2", "claude"), "claude");
        assert_eq!(unpark("claude reviewer", "claude"), "claude reviewer");
        assert_eq!(unpark("my claude helper", "claude"), "my claude helper");
        assert_eq!(unpark("claude", "claude"), "claude");
        assert_eq!(unpark("anything", ""), "anything");
    }
}
