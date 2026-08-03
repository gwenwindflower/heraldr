#!/bin/sh

set -eu

plugin_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$plugin_root"

package_id=$(cargo pkgid --manifest-path Cargo.toml)
version=${package_id##*#}
version=${version##*@}
version=${version##*:}
expected_version="livery $version"
binary=target/release/livery

is_current() {
	[ -x "$1" ] || return 1
	installed_version=$("$1" --version 2>/dev/null) || return 1
	[ "$installed_version" = "$expected_version" ]
}

if is_current "$binary"; then
	printf 'Livery %s is already built.\n' "$version"
	exit 0
fi

mkdir -p target/release

if path_binary=$(command -v livery 2>/dev/null) && is_current "$path_binary"; then
	cp "$path_binary" "$binary"
	chmod +x "$binary"
	printf 'Reused Livery %s from %s.\n' "$version" "$path_binary"
	exit 0
fi

if command -v cargo-binstall >/dev/null 2>&1; then
	printf 'Downloading Livery %s with Cargo Binstall.\n' "$version"
	if cargo-binstall livery \
		--manifest-path Cargo.toml \
		--install-path target/release \
		--strategies crate-meta-data \
		--no-confirm &&
		is_current "$binary"; then
		exit 0
	fi
	printf 'No compatible release artifact found; building Livery from source.\n' >&2
fi

cargo build --locked --release
if ! is_current "$binary"; then
	printf 'Built binary reported an unexpected version; expected %s.\n' "$expected_version" >&2
	exit 1
fi
