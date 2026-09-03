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
