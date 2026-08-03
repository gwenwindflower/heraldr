#!/usr/bin/env bash

release_init() {
	RELEASE_ISSUES=()
	RELEASE_FIXES=()
	RELEASE_FAILURES=()

	if [[ -t 1 && -z "${NO_COLOR+x}" ]]; then
		RELEASE_BOLD=$'\033[1m'
		RELEASE_BLUE=$'\033[34m'
		RELEASE_GREEN=$'\033[32m'
		RELEASE_YELLOW=$'\033[33m'
		RELEASE_RED=$'\033[31m'
		RELEASE_RESET=$'\033[0m'
	else
		RELEASE_BOLD=''
		RELEASE_BLUE=''
		RELEASE_GREEN=''
		RELEASE_YELLOW=''
		RELEASE_RED=''
		RELEASE_RESET=''
	fi
}

release_section() {
	printf '\n%s%s==> %s%s\n' "$RELEASE_BOLD" "$RELEASE_BLUE" "$1" "$RELEASE_RESET"
}

release_info() {
	printf '%s  •%s %s\n' "$RELEASE_BLUE" "$RELEASE_RESET" "$1"
}

release_success() {
	printf '%s  ✓%s %s\n' "$RELEASE_GREEN" "$RELEASE_RESET" "$1"
}

release_warn() {
	printf '%s  !%s %s\n' "$RELEASE_YELLOW" "$RELEASE_RESET" "$1" >&2
}

release_error() {
	printf '%s  ✗%s %s\n' "$RELEASE_RED" "$RELEASE_RESET" "$1" >&2
}

release_add_issue() {
	RELEASE_ISSUES[${#RELEASE_ISSUES[@]}]="$1"
}

release_add_fix() {
	RELEASE_FIXES[${#RELEASE_FIXES[@]}]="$1"
}

release_add_failure() {
	RELEASE_FAILURES[${#RELEASE_FAILURES[@]}]="$1"
}

release_has_failures() {
	[[ ${#RELEASE_FAILURES[@]} -gt 0 ]]
}

release_prompt() {
	local question="$1"
	local default_answer="$2"
	local hint='y/N'
	local answer

	if [[ "$default_answer" == yes ]]; then
		hint='Y/n'
	fi

	while true; do
		printf '%s%s?%s %s [%s] ' "$RELEASE_BOLD" "$RELEASE_BLUE" "$RELEASE_RESET" "$question" "$hint" >&2
		if ! IFS= read -r answer; then
			answer=''
		fi

		case "$answer" in
		'')
			[[ "$default_answer" == yes ]]
			return
			;;
		y | Y | yes | YES | Yes) return 0 ;;
		n | N | no | NO | No) return 1 ;;
		*) release_warn 'Please answer yes or no.' ;;
		esac
	done
}

release_print_list() {
	local title="$1"
	shift

	printf '%s%s%s\n' "$RELEASE_BOLD" "$title" "$RELEASE_RESET"
	if [[ $# -eq 0 ]]; then
		printf '  None\n'
		return
	fi

	local item
	for item in "$@"; do
		printf '  - %s\n' "$item"
	done
}

release_print_report() {
	local version="${1:-unknown}"

	release_section 'Release report'
	printf '%sVersion:%s %s\n\n' "$RELEASE_BOLD" "$RELEASE_RESET" "$version"
	if [[ ${#RELEASE_ISSUES[@]} -gt 0 ]]; then
		release_print_list 'Issues' "${RELEASE_ISSUES[@]}"
	else
		release_print_list 'Issues'
	fi
	printf '\n'
	if [[ ${#RELEASE_FIXES[@]} -gt 0 ]]; then
		release_print_list 'Fixes' "${RELEASE_FIXES[@]}"
	else
		release_print_list 'Fixes'
	fi
	printf '\n'
	if [[ ${#RELEASE_FAILURES[@]} -gt 0 ]]; then
		release_print_list 'Unresolved' "${RELEASE_FAILURES[@]}"
	else
		release_print_list 'Unresolved'
	fi
}

release_read_cargo_version() {
	awk '
    $0 == "[package]" { package = 1; next }
    package && /^\[/ { exit }
    package && /^[[:space:]]*version[[:space:]]*=/ {
      line = $0
      sub(/^[^"]*"/, "", line)
      sub(/".*$/, "", line)
      print line
      exit
    }
  ' "$RELEASE_REPO_ROOT/Cargo.toml"
}

release_read_plugin_version() {
	awk '
    /^\[/ { exit }
    /^[[:space:]]*version[[:space:]]*=/ {
      line = $0
      sub(/^[^"]*"/, "", line)
      sub(/".*$/, "", line)
      print line
      exit
    }
  ' "$RELEASE_REPO_ROOT/herdr-plugin.toml"
}

release_read_lock_version() {
	awk '
    /^\[\[package\]\]$/ { package = 1; livery = 0; next }
    package && /^name = "livery"$/ { livery = 1; next }
    livery && /^version = / {
      line = $0
      sub(/^[^"]*"/, "", line)
      sub(/".*$/, "", line)
      print line
      exit
    }
  ' "$RELEASE_REPO_ROOT/Cargo.lock"
}

release_write_cargo_version() {
	local version="$1"
	local file="$RELEASE_REPO_ROOT/Cargo.toml"
	local temporary
	temporary="$(mktemp "${file}.tmp.XXXXXX")"

	if ! awk -v version="$version" '
    $0 == "[package]" { package = 1 }
    package && $0 != "[package]" && /^\[/ { package = 0 }
    package && /^[[:space:]]*version[[:space:]]*=/ {
      prefix = $0
      sub(/=.*/, "= \"" version "\"", prefix)
      print prefix
      updated = 1
      next
    }
    { print }
    END { if (!updated) exit 1 }
  ' "$file" >"$temporary"; then
		rm -f "$temporary"
		return 1
	fi

	mv "$temporary" "$file"
}

release_write_plugin_version() {
	local version="$1"
	local file="$RELEASE_REPO_ROOT/herdr-plugin.toml"
	local temporary
	temporary="$(mktemp "${file}.tmp.XXXXXX")"

	if ! awk -v version="$version" '
    BEGIN { root = 1 }
    /^\[/ { root = 0 }
    root && /^[[:space:]]*version[[:space:]]*=/ {
      prefix = $0
      sub(/=.*/, "= \"" version "\"", prefix)
      print prefix
      updated = 1
      next
    }
    { print }
    END { if (!updated) exit 1 }
  ' "$file" >"$temporary"; then
		rm -f "$temporary"
		return 1
	fi

	mv "$temporary" "$file"
}

release_sync_lock_version() {
	(
		cd "$RELEASE_REPO_ROOT" || exit
		cargo metadata --format-version 1 --no-deps >/dev/null
	)
}

release_reconcile_versions() {
	local mode="${1:-prompt}"
	local expected_tag="${2:-}"
	local cargo_version
	local expected_version
	local plugin_version
	local lock_version

	cargo_version="$(release_read_cargo_version)"
	if [[ -z "$cargo_version" ]]; then
		release_add_failure 'Cargo.toml does not declare package.version'
		return 1
	fi
	release_info "Cargo.toml is the version source: $cargo_version"

	if [[ -n "$expected_tag" ]]; then
		expected_version="${expected_tag#v}"
		if [[ "$cargo_version" != "$expected_version" ]]; then
			release_add_issue "Cargo.toml declares $cargo_version; release target is $expected_version"
			release_add_failure "Cargo.toml does not match $expected_tag"
		fi
	fi

	plugin_version="$(release_read_plugin_version)"
	if [[ "$plugin_version" != "$cargo_version" ]]; then
		release_add_issue "herdr-plugin.toml declares ${plugin_version:-no version}; expected $cargo_version"
		if [[ "$mode" == prompt ]] && release_prompt "Sync herdr-plugin.toml to $cargo_version" yes; then
			if release_write_plugin_version "$cargo_version"; then
				release_add_fix 'herdr-plugin.toml was out of sync and was synced'
			else
				release_add_failure 'herdr-plugin.toml could not be synchronized'
			fi
		else
			release_add_failure 'herdr-plugin.toml remains out of sync'
		fi
	fi

	lock_version="$(release_read_lock_version)"
	if [[ "$lock_version" != "$cargo_version" ]]; then
		release_add_issue "Cargo.lock declares ${lock_version:-no version}; expected $cargo_version"
		if [[ "$mode" == prompt ]] && release_prompt "Sync Cargo.lock to $cargo_version" yes; then
			if release_sync_lock_version && [[ "$(release_read_lock_version)" == "$cargo_version" ]]; then
				release_add_fix 'Cargo.lock was out of sync and was synced'
			else
				release_add_failure 'Cargo.lock could not be synchronized'
			fi
		else
			release_add_failure 'Cargo.lock remains out of sync'
		fi
	fi

	! release_has_failures
}

release_require_command() {
	local command_name="$1"
	if ! command -v "$command_name" >/dev/null 2>&1; then
		release_add_failure "Required command is unavailable: $command_name"
		return 1
	fi
}

release_require_clean_worktree() {
	local status
	status="$(git -C "$RELEASE_REPO_ROOT" status --short)"
	if [[ -n "$status" ]]; then
		release_add_issue "Worktree has uncommitted changes: $(printf '%s' "$status" | tr '\n' ';')"
		release_add_failure 'Commit or discard worktree changes before releasing'
		return 1
	fi
}

release_require_main_branch() {
	local branch
	branch="$(git -C "$RELEASE_REPO_ROOT" branch --show-current)"
	if [[ "$branch" != main ]]; then
		release_add_failure "Release commands must run on main; current branch is ${branch:-detached}"
		return 1
	fi
}

release_fetch_main_and_tags() {
	release_info 'Fetching origin/main and release tags'
	git -C "$RELEASE_REPO_ROOT" fetch --quiet origin main --tags
}

release_require_local_contains_remote() {
	if ! git -C "$RELEASE_REPO_ROOT" merge-base --is-ancestor origin/main HEAD; then
		release_add_failure 'Local main does not contain origin/main; rebase before preparing the release'
		return 1
	fi
}

release_require_pushed_head() {
	local local_head
	local remote_head
	local_head="$(git -C "$RELEASE_REPO_ROOT" rev-parse HEAD)"
	remote_head="$(git -C "$RELEASE_REPO_ROOT" rev-parse origin/main)"
	if [[ "$local_head" != "$remote_head" ]]; then
		release_add_failure 'HEAD does not match origin/main; run the preparation script and push first'
		return 1
	fi
}

release_computed_version() {
	local version
	version="$(cd "$RELEASE_REPO_ROOT" && git cliff --bumped-version)"
	version="${version##*$'\n'}"
	if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
		release_error "git-cliff returned an invalid release version: $version"
		return 1
	fi
	printf '%s\n' "$version"
}

release_check_tag_available() {
	local version="$1"
	local remote_status

	if git -C "$RELEASE_REPO_ROOT" show-ref --verify --quiet "refs/tags/$version"; then
		release_add_issue "Local tag already exists: $version"
		release_add_failure "Delete or choose a different local tag before releasing $version"
	fi

	if git -C "$RELEASE_REPO_ROOT" ls-remote --exit-code --tags origin "refs/tags/$version" >/dev/null 2>&1; then
		release_add_issue "GitHub tag already exists: $version"
		release_add_failure "GitHub already contains tag $version"
	else
		remote_status=$?
		if [[ "$remote_status" -ne 2 ]]; then
			release_add_failure "Could not check GitHub for tag $version"
		fi
	fi

	! release_has_failures
}

release_run_check() {
	local label="$1"
	shift

	release_info "$label"
	if (cd "$RELEASE_REPO_ROOT" && "$@"); then
		release_success "$label passed"
		return 0
	fi

	release_add_failure "$label failed"
	return 1
}

release_run_project_checks() {
	local failed=0

	release_section 'Project checks'
	release_run_check 'Cargo formatting' cargo fmt --all --check || failed=1
	release_run_check 'Clippy' cargo clippy --all-targets --all-features --locked -- -D warnings || failed=1
	release_run_check 'Rust tests' cargo test --all-features --locked || failed=1
	release_run_check 'Plugin installer tests' ./tests/install-binary.sh || failed=1
	release_run_check 'Release script tests' ./tests/release-scripts.sh || failed=1
	release_run_check 'GitHub Actions security audit' zizmor . || failed=1
	release_run_check 'GitHub Actions pin verification' pinact run --check --verify-comment || failed=1

	return "$failed"
}

release_commit_version_files() {
	local version="$1"
	local status
	local path
	local unexpected=0

	status="$(git -C "$RELEASE_REPO_ROOT" status --short)"
	if [[ -z "$status" ]]; then
		release_info 'Version files already match the release target; no release commit needed'
		return 0
	fi

	while IFS= read -r line; do
		path="${line:3}"
		case "$path" in
		Cargo.toml | Cargo.lock | herdr-plugin.toml) ;;
		*)
			release_add_failure "Unexpected file changed during release preparation: $path"
			unexpected=1
			;;
		esac
	done <<<"$status"

	if [[ "$unexpected" -ne 0 ]]; then
		return 1
	fi

	git -C "$RELEASE_REPO_ROOT" add -- Cargo.toml Cargo.lock herdr-plugin.toml
	if ! git -C "$RELEASE_REPO_ROOT" commit -m "chore(release): prepare $version"; then
		release_add_failure "Could not create the release preparation commit for $version"
		return 1
	fi

	release_add_fix "Created chore(release): prepare $version"
}

release_render_notes() {
	local version="$1"
	(cd "$RELEASE_REPO_ROOT" && git cliff --unreleased --tag "$version" --strip all)
}
