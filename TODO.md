# Heraldr TODO

## Phase 10: Agent icons and session titles

**Requirements**: nm-R010–nm-R019

### Select agent icons

- [ ] Detect Herdr Agent Icons Max and map supported agents
- [ ] Load icon and title preferences from plugin configuration
- [ ] Preserve user icon overrides and the robot fallback

### Format agent session titles

- [ ] Read normalized titles from Herdr snapshots
- [ ] Sanitize and truncate titles by terminal width
- [x] Verify Herdr's support for styling title segments

### Integrate live tab labels

- [ ] Share label composition between full reconciliation and focused polling
- [ ] Prove title updates, foreground transitions, manual names, and numbering through socket tests
- [ ] Document settings, font setup, and rendering limits

### Review the release candidate

- [ ] Merge implementation worktrees locally into the core branch
- [ ] Run local gates and independent review
- [ ] Open one PR and verify hosted checks and all release build targets
- [ ] Run release rehearsal and report remaining human review steps
