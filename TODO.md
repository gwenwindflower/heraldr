# Heraldr TODO

## Phase 1: Release readiness 🌀

**Requirements**: R001, dev-R001, dev-R008, dev-R009, dev-R012

### Repository provisioning

- [ ] #user Create `gwenwindflower/.github` with CONTRIBUTING, SECURITY, FUNDING, and a profile README
- [ ] #user Create `gwenwindflower/heraldr`, add it as `origin`, and push `main`
- [ ] Run `mise run repo:settings --description "Herdr tab, workspace, and agent chrome driven by the live session" --topics "herdr,herdr-plugin,rust,terminal"` and `mise run repo:labels`

### First green CI

- [ ] Open a throwaway PR with a deliberate clippy failure and confirm the annotation lands on the diff
- [ ] Run `mise run repo:rulesets` once CI has reported on `main`

### First release

- [ ] Run `mise run release:rehearse` and resolve everything it reports
- [ ] #user Cut `v0.0.1` with `mise run release`, then run `mise run release:verify`
- [ ] #user Install on a clean `PATH` with `cargo binstall heraldr --git https://github.com/gwenwindflower/heraldr` and `herdr plugin install gwenwindflower/heraldr`

## Phase 2: Tab tracking recovery 🌀

**Requirements**: nm-R001, nm-R002, nm-R003, nm-R004, nm-R005, nm-R006

### Automatic naming across process interruptions

- [x] Reproduce process inspection and rename failures through the binary's socket boundary
- [x] Preserve ownership and keep polling across temporary failures
- [x] Verify recovery through lazygit, credential helpers, the shell, and subsequent programs while respecting manual names
- [ ] #user Run `mise run check` outside the agent sandbox; the manifest's nested sandbox and the hook sweep of `.codex/config.toml` are blocked inside it
- [x] #user Run `mise run dev:reload`, reset an affected tab with `mise run herdr:reset`, and verify in-tab lazygit and subsequent program tracking

### Session ownership isolation

- [x] Trace the affected tab's ownership record across concurrent session passes
- [x] Isolate ownership by session socket while preserving shared-format records
- [x] Verify independent naming with matching tab IDs and cross-session pruning
- [x] Verify two running watchers and run the Rust checks
