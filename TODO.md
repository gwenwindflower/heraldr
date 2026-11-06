# Heraldr TODO

## Phase 10: Agent icons and session titles

**Requirements**: nm-R010–nm-R023, dev-R029

### Select agent icons

- [x] Detect Herdr Agent Icons Max and map supported agents
- [x] Load icon and title preferences from plugin configuration
- [x] Preserve user icon overrides and the robot fallback
- [x] Install commented default preferences without replacing user files

### Format agent session titles

- [x] Read normalized titles from Herdr snapshots
- [x] Sanitize and truncate titles by terminal width
- [x] Verify Herdr's support for styling title segments

### Integrate live tab labels

- [x] Share label composition between full reconciliation and focused polling
- [x] Prove title updates, foreground transitions, manual names, and numbering through socket tests
- [x] Document settings, font setup, and rendering limits

### Review the release candidate

- [x] Merge implementation worktrees locally into the core branch
- [x] Run local gates and independent review
- [ ] #user Review the single PR, hosted checks, and all release build targets
- [ ] #user Merge the PR and rehearse the release from clean `main`
