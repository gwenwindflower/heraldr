# Heraldr DONE

Shipped Phases, newest first. Each entry keeps the Phase header, Objectives, and Tasks verbatim with the boxes checked, followed by a short narrative of decisions and surprises.

## Phase 9: Rust template setup alignment

**Requirements**: dev-R008, dev-R009, dev-R022, dev-R025, dev-R026, dev-R028

### Complete the shared Rust setup

- [x] Compare shared tasks and Rust setup with _tool
- [x] Align CI audit and publishing dependencies
- [x] Run gates and record intentional template differences

Reviewed _tool at 8eaa822099795ad42fb4e74a5348e71ad2030cf5 and the project-workflows Rust kit. Heraldr already carries the version hook interface, crate bootstrap and OIDC publishing, four release targets, packaging and artifact recovery tasks, problem matchers, weekly CI cache rotation, and shared task test ownership.

Assigned binary uploads to the release environment; the audit dependency is also covered by Phase 8. Added project packaging coverage that verifies archive contents, executable permissions, and checksums. The missing-version-task error identifies the Rust task to restore.

Intentional template differences remain Herdr manifest versioning, binaries in target/release, plugin-owned source compilation fallback, open contribution policy, no Homebrew, and sequential unprefixed CI output for Rust annotations. The full release:check gate passed with 35 Rust tests, retained shell suites, optimized and packaged builds, and workflow audits; the added packaging check passed separately.

## Phase 8: Gate CI on workflow audits

**Requirements**: dev-R026, dev-R027

### Audit before checks and tests

- [x] Gate the check job on the workflow audit job
- [x] Run the zizmor commit hook offline
- [x] Validate workflows with zizmor and pinact

Adopted `_tool` PR #1 (`fix/audit-gate` at `9143cca`). Heraldr already allowed `ci-audit:zizmor` to write its cache, so the remaining changes were the `needs: audit` edge and the offline hook. Tests still depend on checks, so a failed audit skips both. The template's `template.yml` and `:::` task-separator fix apply only to its own maintenance matrix.

## Phase 7: Shared task test ownership

**Requirements**: dev-R024, dev-R025

### Upstream generic task coverage

- [x] Verify equivalent label, ruleset, and recovery coverage in `_tool`
- [x] Remove generic suites and retain project integration tests
- [x] Adopt task-maintenance guidance and the Rust shebang exclusion
- [x] Run the project gates and record upstream ownership

Reviewed `_tool` at `afe5f47`; template commit `e93f20d` owns the label, ruleset, and recovery suites and their shared task fixes. Those tests run in template CI rather than being copied into every project. Heraldr retains version synchronization, crate publication, installer, manifest, and automated task-selection coverage because those tests express its Cargo and Herdr integration contracts. Its shared task implementations remain equivalent to the upstreamed versions, with repository names filled in.

The full local gate passed, including 35 Rust tests, retained shell suites, optimized and packaged builds, and file checks. Adopted wtherdr's Rust exclusion for the executable-shebang hook. The project-workflows guidance requires task changes and tests to evolve together, reusable improvements to flow upstream, and periodic comparison during workflow maintenance or release preparation after a long gap.

## Phase 1: Release readiness ✅

**Requirements**: R001, dev-R001, dev-R008, dev-R009, dev-R012

### Repository provisioning

- [x] #user Create `gwenwindflower/.github` with CONTRIBUTING, SECURITY, FUNDING, and a profile README
  - We're skipping this for now except for CONTRIBUTING and README
- [x] #user Create `gwenwindflower/heraldr`, add it as `origin`, and push `main`
- [x] Run `mise run repo:settings --description "Herdr tab, workspace, and agent chrome driven by the live session" --topics "herdr,herdr-plugin,rust,terminal"` and `mise run repo:labels`

### First green CI

- [x] Open a throwaway PR with a deliberate clippy failure and confirm the annotation lands on the diff
- [x] Run `mise run repo:rulesets` once CI has reported on `main`

### First release

- [x] Run `mise run release:rehearse` and resolve everything it reports
- [x] #user Cut `v0.0.1` with `mise run release`, then run `mise run release:verify`
- [x] #user Fetch release tags and run `mise run release:bootstrap-crate` from the release commit
- [x] #user Configure the crates.io trusted publisher and GitHub `release` environment, then set `CRATES_IO_PUBLISHING=true`
- [x] #user Install on a clean `PATH` with `cargo binstall heraldr --git https://github.com/gwenwindflower/heraldr` and `herdr plugin install gwenwindflower/heraldr`

Winnie confirmed repository provisioning, the first release, crates.io setup, and clean-PATH installation. Shared community files were limited to CONTRIBUTING and README. The active main ruleset requires Check, Audit workflows, and both platform test jobs.

[The annotation probe](https://github.com/gwenwindflower/heraldr/pull/2) exposed mise task prefixes preventing the Rust matcher from recognizing Clippy output. CI uses unprefixed output and sequential mise tasks within each job to preserve multiline diagnostics. [The verification run](https://github.com/gwenwindflower/heraldr/actions/runs/36505250260/job/109205113555) attached both deliberate Clippy errors to the added line at `src/main.rs:80`. After removing the probe, [all four hosted checks passed](https://github.com/gwenwindflower/heraldr/actions/runs/36505364467), including Linux and macOS tests. Local `release:check` passed.

## Phase 2: Tab tracking recovery ✅

**Requirements**: nm-R001, nm-R002, nm-R003, nm-R004, nm-R005, nm-R006

### Automatic naming across process interruptions

- [x] Reproduce process inspection and rename failures through the binary's socket boundary
- [x] Preserve ownership and keep polling across temporary failures
- [x] Verify recovery through lazygit, credential helpers, the shell, and subsequent programs while respecting manual names
- [x] #user Run `mise run check` outside the agent sandbox; the manifest's nested sandbox and the hook sweep of `.codex/config.toml` are blocked inside it
- [x] #user Run `mise run dev:reload`, reset an affected tab with `mise run herdr:reset`, and verify in-tab lazygit and subsequent program tracking

### Session ownership isolation

- [x] Trace the affected tab's ownership record across concurrent session passes
- [x] Isolate ownership by session socket while preserving shared-format records
- [x] Verify independent naming with matching tab IDs and cross-session pruning
- [x] Verify two running watchers and run the Rust checks

Temporary process and rename failures retain ownership so polling can recover. Session sockets isolate ownership even when tab IDs overlap, while manual names remain authoritative. Socket-boundary tests cover recovery and concurrent watchers; Winnie confirmed live lazygit and subsequent program tracking and the host-side gate.

## Phase 4: Tool project compliance ✅

**Requirements**: dev-R008, dev-R011, dev-R013, dev-R015, nm-R007, nm-R008, nm-R009

### Rust and plugin distribution contracts

- [x] Enable pedantic Rust linting and the standard release profile
- [x] Remove Homebrew publishing and document direct installation and Cargo PATH requirements
- [x] Prove installer failures report missing Cargo, unresolved binaries, and version mismatches
- [x] Honor plugin storage directories and reconnect interrupted subscriptions
- [x] Run the local gate and workflow audits
- [x] Run release rehearsal; preflight reports that no `origin` remote is configured

### Live verification

- [x] #user Run `mise run dev:reload` and verify reset, clear, custom icons, and watcher recovery in Herdr

The plugin contract uses PATH entrypoints with exact-version installation, plugin-owned storage, and subscription recovery. Pedantic linting, installer failure tests, and workflow audits passed. The initial rehearsal identified the missing remote, resolved during release readiness. Winnie confirmed reset, clear, custom icons, and watcher recovery in Herdr.

## Phase 6: Portable CI and repository provisioning ✅

**Requirements**: dev-R011, dev-R023, dev-R024

### Portable workflow plumbing

- [x] Replace ripgrep in shell tests and verify without it
- [x] Remove unused Homebrew provisioning and formula generation
- [x] Restrict required-check discovery to CI and test the generated ruleset
- [x] Run gates and record the toolchain simplification audit

Hosted test runners exposed an undeclared ripgrep dependency. Shell suites use standard grep and Bash matching, and all suites passed with ripgrep disabled. Ruleset discovery reads only the latest completed CI push run on main, excluding release jobs attached to the same commit; a regression test checks the generated policy without touching GitHub. The existing jq dependency is declared in mise.

Removed unused Homebrew scaffolding and replaced dry-run publication with Cargo's package verification. `release:check` passed, including 35 Rust tests, shell suites, optimized and packaged builds, and workflow audits. The tooling audit retained the shared mise tasks; future distribution-tool research lives in the tool system's vault notes.

## Phase 5: Crates.io release publishing ✅

**Requirements**: dev-R019, dev-R020, dev-R021, dev-R022

### Verified crate publication

- [x] Add optimized build and crate package verification to local and CI gates
- [x] Test release source and artifact checks before crate publication
- [x] Add confirmed first publication and OIDC release tasks
- [x] Verify gates and document the first-release setup

The existing `test:*` task group carries optimized builds and packaged-crate compilation into CI and both merge gates. The package include list ships only Rust source, Rust tests, icons, metadata, and user documentation. Dry runs accept uncommitted changes so they can verify the release pipeline's version bump; real publication requires a clean checkout at the release tag and all eight binary assets on GitHub.

The confirmed bootstrap task uses local Cargo credentials because crates.io requires an initial token-based publication. `CRATES_IO_PUBLISHING` gates the OIDC job until the crate and trusted publisher exist. The job runs after asset upload, only for release events, with the `release` environment and a SHA-pinned official authentication action. `release:check` passed, including the publication failure cases, 35 Rust tests, source packaging, and workflow audits. Initial publication and external OIDC configuration remain human steps in Phase 1.

## Phase 3: Merge and development task isolation ✅

**Requirements**: dev-R014, dev-R015, dev-R016, dev-R017, dev-R018

### Noninteractive merge gates and local development

- [x] Verify automated task selection excludes interactive commands and duplicate suites
- [x] Select the merge gate using Worktrunk's target and default branch variables
- [x] Clarify simulated installer output and verify local development installs the checkout
- [x] Run the gates and preview both merge targets without merging

The merge ran lint twice because its pre-commit hook and release gate both included it. The test wildcard also selected `test:pretty`, running Rust tests twice and putting an interactive terminal task inside the blocking gate. That task was the likely cause of the reported stall; the original terminal stall was not reproduced. Pretty tasks live under `dev:` with unique `b`/`t` aliases. Prek defaults file checks to `pre-commit`, preventing another sweep during `commit-msg`.

[Worktrunk's hook context](https://worktrunk.dev/hook/#template-variables) supplies both `target` and `default_branch`, so a single pre-merge hook selects `check` or `release:check` without assuming `main`. Both gates passed in about four seconds, including 31 Rust tests and the shell suites. Dry-run previews covered feature targets and default branches named `main` and `trunk`; no merge ran.

Installer tests replace Cargo and Binstall in temporary directories. Their output labels the simulation and prints captured installer logs on failure. Local development uses `cargo install --path .` before linking; [Herdr does not run manifest build commands for local links](https://github.com/herdrdev/herdr/blob/master/docs/next/website/src/content/docs/plugins.mdx#build-commands). The dev command sequence was inspected without reloading the live plugin. Workflow audits accept an explicit `GITHUB_TOKEN` or anonymous API access rather than extracting stored credentials through `gh`.
