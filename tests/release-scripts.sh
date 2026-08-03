#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

assert_contains() {
	local output="$1"
	local expected="$2"
	[[ "$output" == *"$expected"* ]] || fail "expected output to contain: $expected"
}

project_version() {
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
  ' "$1/Cargo.toml"
}

plugin_version() {
	awk -F ' = ' '$1 == "version" { gsub(/"/, "", $2); print $2; exit }' "$1/herdr-plugin.toml"
}

lock_version() {
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
  ' "$1/Cargo.lock"
}

make_fake_commands() {
	local name="$1"
	local cliff_version="$2"
	local fakebin="$sandbox/$name-bin"
	mkdir -p "$fakebin"

	cat >"$fakebin/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

case "$1" in
  metadata)
    version="$(awk '
      $0 == "[package]" { package = 1; next }
      package && /^\[/ { exit }
      package && /^[[:space:]]*version[[:space:]]*=/ {
        line = $0
        sub(/^[^"]*"/, "", line)
        sub(/".*$/, "", line)
        print line
        exit
      }
    ' Cargo.toml)"
    awk -v version="$version" '
      /^\[\[package\]\]$/ { package = 1; livery = 0 }
      package && /^name = "livery"$/ { livery = 1 }
      livery && /^version = / {
        print "version = \"" version "\""
        livery = 0
        next
      }
      { print }
    ' Cargo.lock >Cargo.lock.tmp
    mv Cargo.lock.tmp Cargo.lock
    printf '{}\n'
    ;;
  fmt | clippy | test)
    ;;
  *)
    printf 'unexpected cargo command: %s\n' "$*" >&2
    exit 1
    ;;
esac
SCRIPT

	cat >"$fakebin/git-cliff" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail

if [[ " \$* " == *" --bumped-version "* ]]; then
  printf '%s\n' '$cliff_version'
  exit 0
fi

if [[ " \$* " == *" --unreleased "* ]]; then
  printf '## %s\n\n### Features\n\n* Prepared release notes\n' '$cliff_version'
  exit 0
fi

printf 'unexpected git-cliff command: %s\n' "\$*" >&2
exit 1
SCRIPT

	cat >"$fakebin/gh" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

case "${1:-} ${2:-}" in
  "auth status")
    exit 0
    ;;
  "release create")
    shift 2
    printf '%s\n' "$*" >"$GH_TEST_DIR/release-args"
    : >"$GH_TEST_DIR/release-notes"
    while IFS= read -r line; do
      printf '%s\n' "$line" >>"$GH_TEST_DIR/release-notes"
    done
    : >"$GH_TEST_DIR/release-created"
    printf 'https://github.com/example/livery/releases/tag/%s\n' "$1"
    ;;
  "release view")
    [[ -f "$GH_TEST_DIR/release-created" ]]
    ;;
  *)
    printf 'unexpected gh command: %s\n' "$*" >&2
    exit 1
    ;;
esac
SCRIPT

	cat >"$fakebin/zizmor" <<'SCRIPT'
#!/bin/sh
exit 0
SCRIPT

	cat >"$fakebin/pinact" <<'SCRIPT'
#!/bin/sh
exit 0
SCRIPT

	chmod +x "$fakebin"/*
	printf '%s\n' "$fakebin"
}

make_repo() {
	local name="$1"
	local cargo_version="$2"
	local plugin_version="$3"
	local cargo_lock_version="$4"
	local root="$sandbox/$name"
	local remote="$sandbox/$name.git"

	mkdir -p "$root/scripts/lib" "$root/tests" "$root/.github/workflows"
	cp "$repo_root/scripts/lib/release.sh" "$root/scripts/lib/release.sh"
	cp "$repo_root/scripts/sync-version.sh" "$root/scripts/sync-version.sh"
	cp "$repo_root/scripts/prepare-release.sh" "$root/scripts/prepare-release.sh"
	cp "$repo_root/scripts/publish-release.sh" "$root/scripts/publish-release.sh"

	cat >"$root/Cargo.toml" <<TOML
[package]
name = "livery"
version = "$cargo_version"
edition = "2024"
TOML

	cat >"$root/Cargo.lock" <<TOML
version = 4

[[package]]
name = "livery"
version = "$cargo_lock_version"
TOML

	cat >"$root/herdr-plugin.toml" <<TOML
id = "livery"
name = "Livery"
version = "$plugin_version"
TOML

	cat >"$root/tests/install-binary.sh" <<'SCRIPT'
#!/bin/sh
exit 0
SCRIPT
	cat >"$root/tests/release-scripts.sh" <<'SCRIPT'
#!/bin/sh
exit 0
SCRIPT
	chmod +x "$root/tests/install-binary.sh" "$root/tests/release-scripts.sh" "$root/scripts"/*.sh
	: >"$root/cliff.toml"

	git init -q --bare "$remote"
	git --git-dir="$remote" symbolic-ref HEAD refs/heads/main
	git -C "$root" init -q -b main
	git -C "$root" config user.name 'Release Test'
	git -C "$root" config user.email 'release-test@example.com'
	git -C "$root" config commit.gpgsign false
	git -C "$root" remote add origin "$remote"
	git -C "$root" add .
	git -C "$root" commit -q -m 'feat: initial commit'
	git -C "$root" push -q -u origin main
	printf '%s\n' "$root"
}

run_script() {
	local root="$1"
	local fakebin="$2"
	local input="$3"
	local script="$4"
	(
		cd "$root"
		printf '%b' "$input" | GH_TEST_DIR="$root" PATH="$fakebin:/usr/bin:/bin" "$script"
	) 2>&1
}

sync_root="$(make_repo sync 1.2.3 1.2.2 1.2.1)"
sync_bin="$(make_fake_commands sync v1.2.4)"
sync_output="$(run_script "$sync_root" "$sync_bin" '\n\n' ./scripts/sync-version.sh)"
[[ "$(plugin_version "$sync_root")" == 1.2.3 ]] || fail 'plugin manifest was not synchronized'
[[ "$(lock_version "$sync_root")" == 1.2.3 ]] || fail 'Cargo.lock was not synchronized'
assert_contains "$sync_output" 'herdr-plugin.toml was out of sync and was synced'
assert_contains "$sync_output" 'Cargo.lock was out of sync and was synced'

decline_root="$(make_repo decline 1.2.3 1.2.2 1.2.1)"
decline_bin="$(make_fake_commands decline v1.2.4)"
set +e
decline_output="$(run_script "$decline_root" "$decline_bin" 'n\nn\n' ./scripts/sync-version.sh)"
decline_status=$?
set -e
[[ "$decline_status" -ne 0 ]] || fail 'declining synchronization should fail'
[[ "$(plugin_version "$decline_root")" == 1.2.2 ]] || fail 'declined plugin synchronization changed the file'
assert_contains "$decline_output" 'herdr-plugin.toml remains out of sync'

dirty_root="$(make_repo dirty 0.0.1 0.0.1 0.0.1)"
dirty_bin="$(make_fake_commands dirty v0.0.2)"
printf '\n' >>"$dirty_root/Cargo.toml"
set +e
dirty_output="$(run_script "$dirty_root" "$dirty_bin" '' ./scripts/prepare-release.sh)"
dirty_status=$?
set -e
[[ "$dirty_status" -ne 0 ]] || fail 'release preparation should reject a dirty worktree'
assert_contains "$dirty_output" 'Worktree has uncommitted changes'
[[ "$(project_version "$dirty_root")" == 0.0.1 ]] || fail 'dirty preflight changed Cargo.toml'

release_root="$(make_repo release 0.0.1 0.0.1 0.0.1)"
release_bin="$(make_fake_commands release v0.0.2)"
prepare_output="$(run_script "$release_root" "$release_bin" '\n\n\ny\n' ./scripts/prepare-release.sh)"
[[ "$(project_version "$release_root")" == 0.0.2 ]] || fail 'prepare did not bump Cargo.toml'
[[ "$(plugin_version "$release_root")" == 0.0.2 ]] || fail 'prepare did not synchronize the plugin manifest'
[[ "$(lock_version "$release_root")" == 0.0.2 ]] || fail 'prepare did not synchronize Cargo.lock'
[[ "$(git -C "$release_root" log -1 --format=%s)" == 'chore(release): prepare v0.0.2' ]] || fail 'prepare did not create the release commit'
[[ -z "$(git -C "$release_root" status --porcelain)" ]] || fail 'prepare did not leave a clean worktree'
[[ "$(git -C "$release_root" rev-parse HEAD)" == "$(git -C "$release_root" rev-parse origin/main)" ]] || fail 'prepare did not push after confirmation'
assert_contains "$prepare_output" 'Cargo.toml was bumped from 0.0.1 to 0.0.2'
assert_contains "$prepare_output" 'Release notes for v0.0.2'
assert_contains "$prepare_output" 'Prepared release notes'
assert_contains "$prepare_output" 'Release script tests passed'

publish_output="$(run_script "$release_root" "$release_bin" 'y\n' ./scripts/publish-release.sh)"
[[ -f "$release_root/release-created" ]] || fail 'publish did not create the GitHub release'
assert_contains "$(<"$release_root/release-args")" 'v0.0.2 --title v0.0.2 --notes-file -'
assert_contains "$(<"$release_root/release-notes")" 'Prepared release notes'
assert_contains "$publish_output" 'Created GitHub release v0.0.2'

printf 'Release script tests passed.\n'
