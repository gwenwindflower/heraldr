#!/usr/bin/env bash

set -euo pipefail

RELEASE_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$RELEASE_REPO_ROOT/scripts/lib/release.sh"

mode=prompt
expected_tag=''

usage() {
	printf 'Usage: %s [--check] [vVERSION]\n' "$0"
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	--check)
		mode=check
		;;
	-h | --help)
		usage
		exit 0
		;;
	v*)
		if [[ -n "$expected_tag" ]]; then
			usage >&2
			exit 2
		fi
		if [[ ! "$1" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
			usage >&2
			exit 2
		fi
		expected_tag="$1"
		;;
	*)
		usage >&2
		exit 2
		;;
	esac
	shift
done

release_init
release_section 'Version synchronization'

if ! release_reconcile_versions "$mode" "$expected_tag"; then
	release_print_report "${expected_tag:-$(release_read_cargo_version)}"
	exit 1
fi

release_print_report "${expected_tag:-v$(release_read_cargo_version)}"
release_success 'Every version reference matches Cargo.toml'
