#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sandbox="$(mktemp -d "${TMPDIR:-/tmp}/heraldr-installer.XXXXXX")"
trap 'rm -rf "$sandbox"' EXIT

make_case() {
	local name="$1"
	local root="$sandbox/$name"
	mkdir -p "$root/scripts" "$root/fakebin"
	cp "$repo_root/scripts/install-binary.sh" "$root/scripts/install-binary.sh"
	printf '%s\n' '[package]' 'name = "heraldr"' 'version = "0.0.1"' >"$root/Cargo.toml"
	: >"$root/install.log"
	printf '0\n' >"$root/binstall-exit"
	printf '%s\n' "$root"
}

write_heraldr() {
	local path="$1"
	local version="$2"
	cat >"$path" <<SCRIPT
#!/bin/sh
printf '%s\\n' 'heraldr $version'
SCRIPT
	chmod +x "$path"
}

write_cargo() {
	local root="$1"
	cat >"$root/fakebin/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail

case "$1" in
  pkgid)
    printf '%s\n' 'path+file:///tmp/heraldr#heraldr@0.0.1'
    ;;
  install)
    printf '%s\n' "$*" >>"$TEST_LOG"
    cat >"$TEST_BIN/heraldr" <<'BINARY'
#!/bin/sh
printf '%s\n' 'heraldr 0.0.1'
BINARY
    chmod +x "$TEST_BIN/heraldr"
    ;;
  *)
    printf 'Unexpected Cargo command: %s\n' "$*" >&2
    exit 1
    ;;
esac
SCRIPT
	chmod +x "$root/fakebin/cargo"
}

write_binstall() {
	local root="$1"
	local exit_code="${2:-0}"
	cat >"$root/fakebin/cargo-binstall" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >>"$TEST_LOG"
if [[ "$TEST_BINSTALL_EXIT" != 0 ]]; then
  exit "$TEST_BINSTALL_EXIT"
fi
cat >"$TEST_BIN/heraldr" <<'BINARY'
#!/bin/sh
printf '%s\n' 'heraldr 0.0.1'
BINARY
chmod +x "$TEST_BIN/heraldr"
SCRIPT
	chmod +x "$root/fakebin/cargo-binstall"
	printf '%s\n' "$exit_code" >"$root/binstall-exit"
}

run_installer() {
	local root="$1"
	(
		cd "$root"
		TEST_BIN="$root/fakebin" TEST_BINSTALL_EXIT="$(<"$root/binstall-exit")" TEST_LOG="$root/install.log" PATH="$root/fakebin:/usr/bin:/bin" ./scripts/install-binary.sh
	)
}

current_root="$(make_case current)"
write_cargo "$current_root"
write_heraldr "$current_root/fakebin/heraldr" 0.0.1
write_binstall "$current_root"
run_installer "$current_root"
[[ ! -s "$current_root/install.log" ]]

install_root="$(make_case install)"
write_cargo "$install_root"
write_binstall "$install_root"
run_installer "$install_root"
[[ "$(<"$install_root/install.log")" == 'heraldr --manifest-path Cargo.toml --strategies crate-meta-data --locked --force --no-confirm' ]]
[[ "$("$install_root"/fakebin/heraldr --version)" == 'heraldr 0.0.1' ]]
[[ ! -e "$install_root/target/release/heraldr" ]]

source_root="$(make_case source)"
write_cargo "$source_root"
run_installer "$source_root" >"$source_root/output.log" 2>&1
[[ "$(<"$source_root/install.log")" == 'install --path . --locked --force' ]]
rg -q 'Cargo Binstall.*https://github.com/gwenwindflower/heraldr#install' "$source_root/output.log"

fallback_root="$(make_case fallback)"
write_cargo "$fallback_root"
write_binstall "$fallback_root" 1
run_installer "$fallback_root" >"$fallback_root/output.log" 2>&1
[[ "$(<"$fallback_root/install.log")" == $'heraldr --manifest-path Cargo.toml --strategies crate-meta-data --locked --force --no-confirm\ninstall --path . --locked --force' ]]

missing_cargo_root="$(make_case missing-cargo)"
if (cd "$missing_cargo_root" && PATH="$missing_cargo_root/fakebin" ./scripts/install-binary.sh) >"$missing_cargo_root/output.log" 2>&1; then
	printf 'Installer succeeded without Cargo.\n' >&2
	exit 1
fi
rg -q 'Cargo.*https://github.com/gwenwindflower/heraldr#install' "$missing_cargo_root/output.log"

printf 'Binary installer tests passed.\n'
