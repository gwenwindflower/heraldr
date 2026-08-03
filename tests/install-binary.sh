#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT
project_version="$(awk '
  $0 == "[package]" { package = 1; next }
  package && /^\[/ { exit }
  package && /^[[:space:]]*version[[:space:]]*=/ {
    line = $0
    sub(/^[^"]*"/, "", line)
    sub(/".*$/, "", line)
    print line
    exit
  }
' "$repo_root/Cargo.toml")"

make_case() {
	local name="$1"
	local root="$sandbox/$name"
	mkdir -p "$root/scripts" "$root/fakebin"
	cp "$repo_root/scripts/install-binary.sh" "$root/scripts/install-binary.sh"
	: >"$root/install.log"

	cat >"$root/fakebin/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

case "$1" in
  pkgid)
    printf 'path+file:///tmp/livery#livery@%s\n' "$TEST_VERSION"
    ;;
  build)
    printf '%s\n' cargo-build >> "$TEST_LOG"
    mkdir -p target/release
    cat > target/release/livery <<BINARY
#!/bin/sh
printf '%s\n' 'livery $TEST_VERSION'
BINARY
    chmod +x target/release/livery
    ;;
  *)
    printf 'unexpected cargo command: %s\n' "$*" >&2
    exit 1
    ;;
esac
SCRIPT
	chmod +x "$root/fakebin/cargo"
	printf '%s\n' "$root"
}

write_livery() {
	local path="$1"
	local version="$2"
	mkdir -p "$(dirname "$path")"
	cat >"$path" <<SCRIPT
#!/bin/sh
printf '%s\\n' 'livery $version'
SCRIPT
	chmod +x "$path"
}

write_binstall() {
	local path="$1"
	local exit_code="${2:-0}"
	cat >"$path" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' cargo-binstall >> "\$TEST_LOG"
if [[ "$exit_code" != 0 ]]; then
  exit "$exit_code"
fi
while [[ \$# -gt 0 ]]; do
  if [[ \$1 == --install-path ]]; then
    install_path="\$2"
    break
  fi
  shift
done
mkdir -p "\$install_path"
cat > "\$install_path/livery" <<BINARY
#!/bin/sh
printf '%s\\n' 'livery \$TEST_VERSION'
BINARY
chmod +x "\$install_path/livery"
SCRIPT
	chmod +x "$path"
}

run_installer() {
	local root="$1"
	(
		cd "$root"
		TEST_LOG="$root/install.log" TEST_VERSION="$project_version" PATH="$root/fakebin:/usr/bin:/bin" ./scripts/install-binary.sh
	)
}

current_root="$(make_case current)"
write_livery "$current_root/target/release/livery" "$project_version"
write_binstall "$current_root/fakebin/cargo-binstall"
run_installer "$current_root"
[[ ! -s "$current_root/install.log" ]]

path_root="$(make_case path)"
write_livery "$path_root/fakebin/livery" "$project_version"
write_binstall "$path_root/fakebin/cargo-binstall"
run_installer "$path_root"
[[ ! -s "$path_root/install.log" ]]
[[ "$("$path_root/target/release/livery" --version)" == "livery $project_version" ]]

binstall_root="$(make_case binstall)"
write_binstall "$binstall_root/fakebin/cargo-binstall"
run_installer "$binstall_root"
[[ "$(<"$binstall_root/install.log")" == cargo-binstall ]]
[[ "$("$binstall_root/target/release/livery" --version)" == "livery $project_version" ]]

stale_root="$(make_case stale)"
write_livery "$stale_root/target/release/livery" 0.0.0
write_livery "$stale_root/fakebin/livery" 0.0.0
write_binstall "$stale_root/fakebin/cargo-binstall"
run_installer "$stale_root"
[[ "$(<"$stale_root/install.log")" == cargo-binstall ]]
[[ "$("$stale_root/target/release/livery" --version)" == "livery $project_version" ]]

fallback_root="$(make_case fallback)"
write_binstall "$fallback_root/fakebin/cargo-binstall" 1
run_installer "$fallback_root"
[[ "$(<"$fallback_root/install.log")" == $'cargo-binstall\ncargo-build' ]]

source_root="$(make_case source)"
run_installer "$source_root"
[[ "$(<"$source_root/install.log")" == cargo-build ]]
