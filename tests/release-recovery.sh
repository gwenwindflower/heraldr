#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sandbox="$(mktemp -d "${TMPDIR:-/tmp}/heraldr-recovery-test.XXXXXX")"
trap 'rm -rf "$sandbox"' EXIT
mkdir -p "$sandbox/bin"
cat >"$sandbox/bin/gh" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
case "$1 $2" in
  'api repos/gwenwindflower/heraldr/actions/runs/123')
    printf '%s\tcompleted\t.github/workflows/release-build.yml\tv0.0.1\tabc\n' "${TEST_EVENT:-release}" ;;
  'api repos/gwenwindflower/heraldr/commits/v0.0.1')
    printf '%s\n' "${TEST_COMMIT:-abc}" ;;
  'release view') printf 'false\n' ;;
  'run download')
    target="${7#heraldr-}"
    archive="heraldr-$target-v0.0.1.tgz"
    printf 'test binary\n' >"$9/$archive"
    (cd "$9" && shasum -a 256 "$archive" >"$archive.sha256")
    if [[ "${TEST_CORRUPT:-0}" == 1 ]]; then printf 'corrupt\n' >>"$9/$archive"; fi ;;
  'release upload')
    [[ "$3" == v0.0.1 && "$4" == --repo && "$5" == gwenwindflower/heraldr && "$#" == 13 ]]
    printf 'uploaded\n' >"$TEST_UPLOAD" ;;
  *) printf 'Unexpected command: %s\n' "$*" >&2; exit 1 ;;
esac
SCRIPT
chmod +x "$sandbox/bin/gh"
export TEST_UPLOAD="$sandbox/upload"
run_recovery() {
  PATH="$sandbox/bin:$PATH" bash "$repo_root/mise-tasks/release/recover-assets" 123
}
run_recovery
[[ -f "$TEST_UPLOAD" ]]
rm "$TEST_UPLOAD"
for failure in TEST_EVENT=workflow_dispatch TEST_COMMIT=wrong TEST_CORRUPT=1; do
  export "${failure?}"
  if run_recovery >"$sandbox/output" 2>&1; then
    printf 'Recovery accepted %s\n' "$failure" >&2
    exit 1
  fi
  [[ ! -e "$TEST_UPLOAD" ]]
  unset "${failure%%=*}"
done
printf 'Release artifact recovery tests passed.\n'
