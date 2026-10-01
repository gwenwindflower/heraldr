#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sandbox="$(mktemp -d "${TMPDIR:-/tmp}/heraldr-packaging.XXXXXX")"
trap 'rm -rf "$sandbox"' EXIT
mkdir -p "$sandbox/bin" "$sandbox/mise-tasks/version" "$sandbox/target/release"
cp "$repo_root/mise-tasks/version/read" "$sandbox/mise-tasks/version/read"
cp "$repo_root/Cargo.toml" "$sandbox/Cargo.toml"
printf '#!/bin/sh\n[ "$*" = "run build" ]\n' >"$sandbox/bin/mise"
printf '#!/bin/sh\nprintf "heraldr fixture\\n"\n' >"$sandbox/target/release/heraldr"
chmod +x "$sandbox/bin/mise" "$sandbox/target/release/heraldr"
MISE_PROJECT_ROOT="$sandbox" PATH="$sandbox/bin:$PATH" bash "$repo_root/mise-tasks/release/package" aarch64-apple-darwin
package="heraldr-aarch64-apple-darwin-v$(MISE_PROJECT_ROOT="$sandbox" "$sandbox/mise-tasks/version/read")"
(cd "$sandbox/dist" && shasum -a 256 -c "$package.tgz.sha256")
mkdir "$sandbox/unpacked"
tar -C "$sandbox/unpacked" -xzf "$sandbox/dist/$package.tgz"
cmp "$sandbox/target/release/heraldr" "$sandbox/unpacked/$package/heraldr"
[[ -x "$sandbox/unpacked/$package/heraldr" ]] || exit 1
[[ ! -d "$sandbox/dist/$package" ]] || exit 1
printf 'Release packaging tests passed.\n'
