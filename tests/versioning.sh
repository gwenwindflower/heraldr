#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sandbox="$(mktemp -d "${TMPDIR:-/tmp}/heraldr-versioning.XXXXXX")"
trap 'rm -rf "$sandbox"' EXIT

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

assert_contains() {
	[[ "$1" == *"$2"* ]] || fail "expected output to contain: $2"
}

fakebin="$sandbox/bin"
mkdir -p "$fakebin"

# A fake cargo that only knows `update --workspace --offline` and rewrites the lockfile entry.
cat >"$fakebin/cargo" <<'FAKE'
#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != update || "${2:-}" != --workspace ]]; then
  printf 'unexpected cargo command: %s\n' "$*" >&2
  exit 1
fi

version="$(awk '
  $0 == "[package]" { package = 1; next }
  package && /^\[/ { exit }
  package && /^[[:space:]]*version[[:space:]]*=/ { sub(/^[^"]*"/, ""); sub(/".*$/, ""); print; exit }
' Cargo.toml)"
awk -v version="$version" '
  /^\[\[package\]\]$/ { heraldr = 0 }
  /^name = "heraldr"$/ { heraldr = 1; print; next }
  heraldr && /^version = / { print "version = \"" version "\""; heraldr = 0; next }
  { print }
' Cargo.lock >Cargo.lock.fake
mv Cargo.lock.fake Cargo.lock
FAKE
chmod +x "$fakebin/cargo"

make_repo() {
	local name="$1" cargo_version="$2" lock_version="$3" manifest_version="$4"
	local root="$sandbox/$name"

	mkdir -p "$root/mise-tasks"
	cp -R "$repo_root/mise-tasks/version" "$root/mise-tasks/version"
	cp -R "$repo_root/mise-tasks/release" "$root/mise-tasks/release"

	cat >"$root/Cargo.toml" <<TOML
[package]
name = "heraldr"
version = "$cargo_version"
edition = "2024"

[dependencies]
version = "1.0"
TOML

	cat >"$root/Cargo.lock" <<TOML
version = 4

[[package]]
name = "anyhow"
version = "1.0.99"

[[package]]
name = "heraldr"
version = "$lock_version"
TOML

	cat >"$root/herdr-plugin.toml" <<TOML
id = "heraldr"
name = "Heraldr"
version = "$manifest_version"
min_herdr_version = "0.7.5"

[[build]]
version = "ignored"
TOML

	printf '%s\n' "$root"
}

run_task() {
	local root="$1" task="$2"
	shift 2
	(cd "$root" && PATH="$fakebin:/usr/bin:/bin" MISE_PROJECT_ROOT="$root" "$root/mise-tasks/$task" "$@") 2>&1
}

read_field() {
	local file="$1"
	awk '/^version = / { sub(/^[^"]*"/, ""); sub(/".*$/, ""); print; exit }' "$file"
}

synced="$(make_repo synced 1.2.3 1.2.3 1.2.3)"
[[ "$(run_task "$synced" version/read)" == 1.2.3 ]] || fail 'version/read misread Cargo.toml'
run_task "$synced" version/check >/dev/null || fail 'synchronized versions reported drift'
run_task "$synced" version/check v1.2.3 >/dev/null || fail 'matching tag reported drift'

set +e
mismatch="$(run_task "$synced" version/check v9.9.9)"
mismatch_status=$?
set -e
[[ "$mismatch_status" -ne 0 ]] || fail 'a tag Cargo does not declare should fail'
assert_contains "$mismatch" 'declares 1.2.3; v9.9.9 expects 9.9.9'

drifted="$(make_repo drifted 1.2.3 1.2.1 1.2.2)"
set +e
drift="$(run_task "$drifted" version/check)"
drift_status=$?
set -e
[[ "$drift_status" -ne 0 ]] || fail 'version drift should fail'
assert_contains "$drift" 'Cargo.lock records 1.2.1'
assert_contains "$drift" 'herdr-plugin.toml declares 1.2.2'

run_task "$drifted" version/sync >/dev/null || fail 'version/sync left the repository out of sync'
[[ "$(awk '/^name = "heraldr"$/ { found = 1; next } found && /^version = / { sub(/^[^"]*"/, ""); sub(/".*$/, ""); print; exit }' "$drifted/Cargo.lock")" == 1.2.3 ]] || fail 'version/sync did not repair Cargo.lock'
[[ "$(read_field "$drifted/herdr-plugin.toml")" == 1.2.3 ]] || fail 'version/sync did not repair herdr-plugin.toml'
[[ "$(run_task "$drifted" version/read)" == 1.2.3 ]] || fail 'version/sync changed the source of truth'
[[ -z "$(find "$drifted" -name '*.next')" ]] || fail 'version/sync left a temporary file behind'

bump="$(make_repo bump 1.2.3 1.2.3 1.2.3)"
run_task "$bump" version/bump v1.3.0 >/dev/null || fail 'version/bump failed'
[[ "$(run_task "$bump" version/read)" == 1.3.0 ]] || fail 'version/bump did not set Cargo.toml'
[[ "$(read_field "$bump/herdr-plugin.toml")" == 1.3.0 ]] || fail 'version/bump did not sync herdr-plugin.toml'
run_task "$bump" version/check v1.3.0 >/dev/null || fail 'version/bump left the repository unreleasable'

[[ "$(run_task "$bump" version/files | tr '\n' ' ')" == 'Cargo.toml Cargo.lock herdr-plugin.toml ' ]] || fail 'version/files must list all three version files'

printf 'Versioning tests passed.\n'
