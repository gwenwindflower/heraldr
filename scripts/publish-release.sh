#!/usr/bin/env bash

set -euo pipefail

RELEASE_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$RELEASE_REPO_ROOT/scripts/lib/release.sh"

release_init
release_section 'Publish release'

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
release_require_pushed_head || true

if release_has_failures; then
	release_print_report 'unknown'
	exit 1
fi

VERSION="$(release_computed_version)"
release_info "git-cliff selected $VERSION"
release_check_tag_available "$VERSION" || true
release_reconcile_versions check "$VERSION" || true

if release_has_failures; then
	release_print_report "$VERSION"
	exit 1
fi

release_run_project_checks || true
if release_has_failures; then
	release_print_report "$VERSION"
	exit 1
fi

if ! release_notes="$(release_render_notes "$VERSION")"; then
	release_add_failure "git-cliff could not render release notes for $VERSION"
	release_print_report "$VERSION"
	exit 1
fi

release_print_report "$VERSION"
release_section "Release notes for $VERSION"
printf '%s\n' "$release_notes"

if ! release_prompt "Create GitHub release $VERSION" no; then
	release_warn 'Release creation skipped.'
	exit 0
fi

release_section 'Publish'
if printf '%s\n' "$release_notes" | gh release create "$VERSION" --title "$VERSION" --notes-file -; then
	release_success "Created GitHub release $VERSION"
else
	release_error "GitHub release creation failed for $VERSION"
	exit 1
fi
