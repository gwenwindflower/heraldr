# Releasing Livery

`Cargo.toml` is the source of truth for Livery's version. The release scripts synchronize `Cargo.lock` and `herdr-plugin.toml`, validate the repository, and keep release notes in the GitHub Release rather than a separate changelog file.

## Requirements

Release commands run from `main` and require Cargo, git-cliff, the GitHub CLI, zizmor, and pinact. Authenticate `gh` before starting.

All release tags use `vMAJOR.MINOR.PATCH`. The policy in `cliff.toml` begins at `v0.0.1`, keeps `0.0.x` releases on patch bumps, and does not promote the project to `0.1.0` or `1.0.0` automatically.

## Synchronize versions

Run the interactive synchronizer whenever a non-Cargo version reference may have drifted:

```bash
./scripts/sync-version.sh
```

Cargo's package version remains unchanged. Each mismatched file is reported and offered for repair with a `Y/n` prompt. CI uses the read-only form:

```bash
./scripts/sync-version.sh --check
```

An optional tag also verifies that Cargo matches a specific release:

```bash
VERSION="$(git cliff --bumped-version)"
./scripts/sync-version.sh --check "$VERSION"
```

## Prepare a release

Start with every intended release change committed on a clean local `main`, then run:

```bash
./scripts/prepare-release.sh
```

Preparation:

1. Fetches `origin/main` and existing tags, then confirms the worktree is clean and local `main` contains the remote branch.
2. Captures the version from `git cliff --bumped-version` and confirms that neither the local nor GitHub tag exists.
3. Offers to update `Cargo.toml`, then reconcile `Cargo.lock` and `herdr-plugin.toml` to Cargo's version.
4. Runs Rust checks, installer and release-script tests, zizmor, and pinact.
5. Commits version changes as `chore(release): prepare <tag>` and confirms the worktree is clean again.
6. Prints a report of detected issues, applied fixes, and unresolved failures.
7. Prints `git cliff --unreleased --tag <tag> --strip all` for review, then asks whether to push `main` with a default of no.

Declining the final prompt leaves the clean release commit locally. Rerun the command when ready to review and push it.

## Publish a release

After the preparation commit and every intended release commit are on `origin/main`, run:

```bash
./scripts/publish-release.sh
```

Publication requires a clean worktree whose `HEAD` exactly matches `origin/main`. It recomputes the git-cliff version, verifies synchronized versions and unused tags, reruns the project checks, and prints the release notes. Confirming the final `y/N` prompt sends those notes directly to:

```bash
gh release create "$VERSION" --title "$VERSION" --notes-file -
```

GitHub creates the tag at the default-branch head when it publishes the release. Do not create or push a separate tag. The `Release build` workflow then verifies the tag against Cargo, builds each supported target, and attaches archives and checksums.

## Verify the release

Inspect the release, workflow, and uploaded assets:

```bash
VERSION="$(gh release view --json tagName --jq '.tagName')"
gh release view "$VERSION"
gh run list --workflow release-build.yml --limit 5
gh release view "$VERSION" --json assets --jq '.assets[].name'
```

A complete release has one `.tgz` archive and one `.tgz.sha256` file for each supported target. Rerun failed release jobs from their original event so they retain permission to upload assets:

```bash
gh run rerun <run-id> --failed
```

The manual workflow path rebuilds an existing tag for diagnosis and keeps outputs as workflow artifacts without changing the release:

```bash
gh workflow run release-build.yml -f tag="$VERSION"
```
