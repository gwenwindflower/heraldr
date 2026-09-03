#!/bin/sh

set -eu

install_docs=https://github.com/supermodellabs/heraldr#install
if ! command -v cargo >/dev/null 2>&1; then
	printf 'Cargo is required to install Heraldr. Install it before continuing: %s\n' "$install_docs" >&2
	exit 1
fi

plugin_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$plugin_root"

package_id=$(cargo pkgid --manifest-path Cargo.toml)
version=${package_id##*#}
version=${version##*@}
version=${version##*:}
expected_version="heraldr $version"

is_current() {
	[ -x "$1" ] || return 1
	installed_version=$("$1" --version 2>/dev/null) || return 1
	[ "$installed_version" = "$expected_version" ]
}

if path_binary=$(command -v heraldr 2>/dev/null) && is_current "$path_binary"; then
	printf 'Heraldr %s is already installed at %s.\n' "$version" "$path_binary"
	exit 0
fi

if command -v cargo-binstall >/dev/null 2>&1; then
	printf 'Downloading Heraldr %s with Cargo Binstall.\n' "$version"
	if cargo-binstall heraldr \
		--manifest-path Cargo.toml \
		--strategies crate-meta-data \
		--locked \
		--force \
		--no-confirm &&
		path_binary=$(command -v heraldr 2>/dev/null) &&
		is_current "$path_binary"; then
		exit 0
	fi
	printf 'No compatible release artifact found; installing Heraldr from source.\n' >&2
else
	printf 'Cargo Binstall is unavailable; installing Heraldr from source. Install Cargo Binstall for faster installs: %s\n' "$install_docs" >&2
fi

cargo install --path . --locked --force
if ! path_binary=$(command -v heraldr 2>/dev/null); then
	printf 'Cargo installed Heraldr, but it is not on PATH. Add $CARGO_HOME/bin (normally ~/.cargo/bin) to PATH.\n' >&2
	exit 1
fi
if ! is_current "$path_binary"; then
	printf 'Heraldr on PATH reported an unexpected version; expected %s.\n' "$expected_version" >&2
	exit 1
fi
