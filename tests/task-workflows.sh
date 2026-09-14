#!/usr/bin/env bash

set -euo pipefail
cd "${MISE_PROJECT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

for task in check test 'test:*' release:check; do
	output="$(mise run --dry-run "$task" 2>&1)"
	if [[ "$output" == *'cargo pretty'* ]]; then
		fail "$task includes interactive Cargo output"
	fi
	if [[ "$output" == *'cargo install'* || "$output" == *'cargo binstall'* || "$output" == *'herdr plugin install'* ]]; then
		fail "$task installs Heraldr instead of testing it"
	fi
	count="$(printf '%s\n' "$output" | rg -c 'cargo test --all-features --locked' || true)"
	[[ "$count" == 1 ]] || fail "$task must run the Rust suite exactly once (found ${count:-0})"
	if [[ "$task" == check || "$task" == release:check ]]; then
		count="$(printf '%s\n' "$output" | rg -c 'prek run --all-files' || true)"
		[[ "$count" == 1 ]] || fail "$task must run the hook sweep exactly once (found ${count:-0})"
	fi
done

printf 'Automated task selection tests passed.\n'
