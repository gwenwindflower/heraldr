# Release and repository plumbing

## Goals

Every release is cut from a clean `main` by a human-gated pipeline that a contributor can rehearse without side effects. The version has one source of truth, CI proves the tree before anything ships, and the published archives are what `herdr plugin install` and `cargo binstall` resolve. Non-goals: nightly or pre-release channels, Homebrew, and crates.io until a user asks.

## Requirements

- **dev-R001** — Always: `Cargo.toml` is the sole source of truth for the version, and `Cargo.lock` and `herdr-plugin.toml` match it before a release is created.
- **dev-R002** — If the release tag already exists locally or on GitHub, then `release:preflight` fails and names the tag.
- **dev-R003** — If the worktree is dirty, the branch is not `main`, or `main` is behind `origin/main`, then `release:preflight` fails and lists every problem it found.
- **dev-R004** — If `cliff.toml` names a different `owner/repo` than the `origin` remote, then `release:preflight` fails.
- **dev-R005** — When `release:commit` runs, only the files `version:files` reports may be modified; any other change aborts the release.
- **dev-R006** — Always: the GitHub release is created from the head of `origin/main`, and GitHub creates the tag; no tag is pushed separately.
- **dev-R007** — When a release is published, the build workflow attaches one `<name>-<target>-v<version>.tgz` per target triple, each with a `.sha256` sidecar.
- **dev-R008** — Always: every `uses:` in `.github/workflows/` is pinned to a commit SHA with a trailing version comment, and `mise run ci-audit` passes.
- **dev-R009** — Always: CI reports lint and test failures as file-and-line annotations on the diff.
- **dev-R010** — While the repository is flagged as a template, CI jobs are skipped.
- **dev-R011** — Never: Heraldr publishes to a Homebrew tap; Herdr's plugin installer is the distribution path.
- **dev-R012** — Always: `mise run release:rehearse` runs every read-only step of the release and writes nothing.
- **dev-R013** — Always: every manifest entrypoint invokes `heraldr` from `PATH`, and the build hook installs the exact declared version there.
- **dev-R014** — Every commit runs file checks at `pre-commit`; `commit-msg` checks only the subject and rejects subjects git-cliff cannot parse.
- **dev-R015** — When a branch merges through `wt merge`, one gate runs after the rebase: `release:check` for the default target branch, otherwise `check`; failure aborts the merge.
- **dev-R016** — Automated checks and CI run each test suite once without interactive tasks or installing Heraldr on the user's `PATH`.
- **dev-R017** — Local development compiles and installs the checkout before linking it into Herdr, without downloading a published Heraldr binary.
- **dev-R018** — Installer tests identify simulated installs in their output and keep Cargo and Binstall replacements inside temporary test directories.
