#!/usr/bin/env bash

set -euo pipefail

RELEASE_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$RELEASE_REPO_ROOT/scripts/lib/release.sh"

release_init
release_section 'Prepare release'

for command_name in git cargo git-cliff gh zizmor pinact; do
	release_require_command "$command_name" || true
done
release_require_main_branch || true
release_require_clean_worktree || true

if release_has_failures; then
	release_print_report 'unknown'
	exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
	release_add_failure 'GitHub CLI authentication is unavailable'
fi
if ! release_fetch_main_and_tags; then
	release_add_failure 'Could not fetch origin/main and release tags'
fi
release_require_local_contains_remote || true

if release_has_failures; then
	release_print_report 'unknown'
	exit 1
fi

VERSION="$(release_computed_version)"
release_info "git-cliff selected $VERSION"
release_check_tag_available "$VERSION" || true

cargo_version="$(release_read_cargo_version)"
target_version="${VERSION#v}"
if [[ "$cargo_version" != "$target_version" ]]; then
	release_add_issue "Cargo.toml declares $cargo_version; git-cliff selected $target_version"
	if release_prompt "Bump Cargo.toml to $target_version" yes; then
		if release_write_cargo_version "$target_version"; then
			release_add_fix "Cargo.toml was bumped from $cargo_version to $target_version"
		else
			release_add_failure 'Cargo.toml could not be updated'
		fi
	else
		release_add_failure "Cargo.toml remains at $cargo_version"
	fi
fi

release_reconcile_versions prompt "$VERSION" || true

if release_has_failures; then
	release_print_report "$VERSION"
	exit 1
fi

release_run_project_checks || true
if release_has_failures; then
	release_print_report "$VERSION"
	exit 1
fi

release_commit_version_files "$VERSION" || true
release_require_clean_worktree || true
if release_has_failures; then
	release_print_report "$VERSION"
	exit 1
fi

release_print_report "$VERSION"
release_section "Release notes for $VERSION"
if ! release_render_notes "$VERSION"; then
	release_add_failure "git-cliff could not render release notes for $VERSION"
	release_print_report "$VERSION"
	exit 1
fi

if release_prompt 'Push main to origin' no; then
	release_section 'Push'
	if git -C "$RELEASE_REPO_ROOT" push origin main; then
		release_success "Pushed $VERSION preparation to origin/main"
	else
		release_error 'Push failed'
		exit 1
	fi
else
	release_warn 'Push skipped; the release preparation commit remains local.'
fi
