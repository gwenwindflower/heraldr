#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
task="$repo_root/mise-tasks/dev/local"
sandbox="$(mktemp -d)"
trap 'rm -rf "$sandbox"' EXIT

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

mkdir -p "$sandbox/bin" "$sandbox/checkout" "$sandbox/other"
checkout="$(cd "$sandbox/checkout" && pwd -P)"
other="$(cd "$sandbox/other" && pwd -P)"

cat >"$sandbox/bin/herdr" <<'EOF'
#!/usr/bin/env bash
if [[ "$1 $2" == "plugin list" ]]; then
	printf '%s\n' "$FAKE_LISTING"
	exit 0
fi
printf 'herdr %s\n' "$*" >>"$FAKE_LOG"
EOF
cat >"$sandbox/bin/mise" <<'EOF'
#!/usr/bin/env bash
printf 'mise %s\n' "$*" >>"$FAKE_LOG"
EOF
chmod +x "$sandbox/bin/herdr" "$sandbox/bin/mise"

listing() {
	local source="$1" root="$2"
	if [[ -z "$source" ]]; then
		printf '{"id":"cli:plugin","result":{"plugins":[],"type":"plugin_list"}}'
		return
	fi
	printf '{"id":"cli:plugin","result":{"plugins":[{"actions":[],"plugin_id":"heraldr","plugin_root":"%s","source":{"kind":"%s"},"version":"0.0.3"}],"type":"plugin_list"}}' "$root" "$source"
}

run_task() {
	: >"$sandbox/log"
	FAKE_LISTING="$(listing "$1" "$2")" FAKE_LOG="$sandbox/log" \
		PATH="$sandbox/bin:$PATH" MISE_PROJECT_ROOT="$checkout" \
		bash "$task" >"$sandbox/out" 2>&1 || fail "dev:local exited nonzero: $(cat "$sandbox/out")"
}

expect_log() {
	local scenario="$1" expected="$2"
	printf '%s' "$expected" >"$sandbox/expected"
	diff -u "$sandbox/expected" "$sandbox/log" >&2 || fail "$scenario ran the wrong steps"
}

swap='mise run install
mise run herdr:link
mise run herdr:restart
'

run_task local "$checkout"
expect_log 'already linked to the calling checkout' ''
grep -Fq 'dev:reload' "$sandbox/out" || fail 'already-linked run does not point at dev:reload'

run_task local "$other"
expect_log 'linked to another checkout' "herdr plugin unlink heraldr
$swap"

run_task local "$sandbox/deleted-worktree"
expect_log 'linked to a deleted checkout' "herdr plugin unlink heraldr
$swap"

run_task github ''
expect_log 'installed from GitHub' "herdr plugin uninstall heraldr
$swap"

run_task '' ''
expect_log 'not installed' "$swap"

printf 'dev:local tests passed.\n'
