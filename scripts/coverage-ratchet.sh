#!/usr/bin/env bash
# coverage-ratchet.sh
#
# Tarpaulin-based untested-lines ratchet. Fails if HEAD has MORE uncovered
# lines than the merge-base (default: origin/main).
#
# Why lines-not-percentage: deleting tested code can raise the uncovered
# percentage artificially; tracking absolute uncovered lines avoids that.
#
# Usage:
#   scripts/coverage-ratchet.sh                  # ratchet vs origin/main merge-base
#   scripts/coverage-ratchet.sh --base <rev>     # ratchet vs <rev>
#   scripts/coverage-ratchet.sh --self-test      # run built-in fixtures, no cargo
#   scripts/coverage-ratchet.sh --summary <json> # print top-N uncovered summary
#
# Wire as pre-push hook (invoked, not installed):
#   ln -s ../../scripts/coverage-ratchet.sh .git/hooks/pre-push
#
# Env overrides:
#   COVERAGE_BASE_REF (default: origin/main)
#   COVERAGE_DIR      (default: <repo>/coverage)
#   COVERAGE_TOP_N    (default: 5)

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
COVERAGE_DIR="${COVERAGE_DIR:-$ROOT/coverage}"
COVERAGE_BASE_REF="${COVERAGE_BASE_REF:-origin/main}"
COVERAGE_TOP_N="${COVERAGE_TOP_N:-5}"
PY="$ROOT/scripts/coverage-ratchet-summary.py"

log() { printf '[coverage-ratchet] %s\n' "$*" >&2; }

uncovered_total() { python3 "$PY" total "$1"; }
summarize_json()  { python3 "$PY" summary "$1" "$ROOT" "$COVERAGE_TOP_N"; }

run_tarpaulin() {
  local out_dir="$1"
  mkdir -p "$out_dir"
  # Isolate build artifacts per measurement so --skip-clean (kept for speed
  # within a single run) cannot leak compiled output from a previous HEAD/base
  # switch into the next measurement. Each out_dir gets its own target/.
  local target_dir="$out_dir/target"
  mkdir -p "$target_dir"
  log "running cargo tarpaulin -> $out_dir (CARGO_TARGET_DIR=$target_dir, up to 75s)"
  ( cd "$ROOT" && CARGO_TARGET_DIR="$target_dir" \
      cargo tarpaulin --skip-clean --ignore-tests \
      --out Json --output-dir "$out_dir" --timeout 120 >&2 ) || return $?
  if [[ -f "$out_dir/tarpaulin-report.json" ]]; then
    printf '%s\n' "$out_dir/tarpaulin-report.json"
  else
    log "tarpaulin report not found in $out_dir"
    return 2
  fi
}

ratchet() {
  local base_ref="$1"
  mkdir -p "$COVERAGE_DIR/head" "$COVERAGE_DIR/base"
  local head_json base_json merge_base head_sha head_total base_total
  # Resolve merge-base and the current HEAD BEFORE any measurement so the
  # stash/checkout ordering is deterministic.
  merge_base=$(git merge-base HEAD "$base_ref" 2>/dev/null || git rev-parse "$base_ref")
  head_sha=$(git rev-parse HEAD)
  log "merge-base vs $base_ref = $merge_base"
  # Measure the BASE first: stash any WIP, switch to merge-base, run tarpaulin
  # in its own CARGO_TARGET_DIR, then return to HEAD and measure there. This
  # ordering guarantees the base measurement is never contaminated by build
  # artifacts produced while on HEAD (the original bug: running tarpaulin on
  # HEAD first and then on base with --skip-clean meant base reused HEAD's
  # incremental build outputs).
  log "stashing and checking out $merge_base"
  git stash push -u -m "coverage-ratchet-wip" >/dev/null 2>&1 || true
  git checkout -q "$merge_base"
  if ! base_json=$(run_tarpaulin "$COVERAGE_DIR/base"); then
    git checkout -q "$head_sha"; git stash pop >/dev/null 2>&1 || true; return 2
  fi
  git checkout -q "$head_sha"
  git stash pop >/dev/null 2>&1 || true
  if ! head_json=$(run_tarpaulin "$COVERAGE_DIR/head"); then
    return 2
  fi
  head_total=$(uncovered_total "$head_json")
  base_total=$(uncovered_total "$base_json")
  {
    echo "## Coverage Ratchet (untested lines)"
    echo ""
    echo "| metric | value |"
    echo "| --- | --- |"
    echo "| base ($merge_base) uncovered lines | $base_total |"
    echo "| HEAD uncovered lines | $head_total |"
    echo "| delta | $((head_total - base_total)) |"
    echo ""
    echo "### Top files by uncovered lines at HEAD"
    echo '```'
    summarize_json "$head_json"
    echo '```'
    echo ""
    echo "Heuristic: unit=pure funcs; integration=I/O, routing, or protocol; property=data transforms."
  } | tee "$COVERAGE_DIR/summary.md" >&2
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    cat "$COVERAGE_DIR/summary.md" >> "$GITHUB_STEP_SUMMARY"
  fi
  if (( head_total > base_total )); then
    log "FAIL: HEAD has $head_total uncovered lines vs base $base_total (delta=+$((head_total - base_total)))"
    return 1
  fi
  log "PASS: uncovered lines did not increase ($head_total <= $base_total)"
}

self_test() {
  local tmp; tmp=$(mktemp -d); trap "rm -rf '$tmp'" EXIT
  cat >"$tmp/base.json" <<'JSON'
{"files":[{"path":["/","src","lib.rs"],"traces":[{"line":1,"stats":{"Line":1}},{"line":2,"stats":{"Line":0}},{"line":3,"stats":{"Line":0}},{"line":4,"stats":{"Line":0}}]}]}
JSON
  cat >"$tmp/head_ok.json" <<'JSON'
{"files":[{"path":["/","src","lib.rs"],"traces":[{"line":1,"stats":{"Line":1}},{"line":2,"stats":{"Line":1}},{"line":3,"stats":{"Line":0}},{"line":4,"stats":{"Line":0}}]}]}
JSON
  cat >"$tmp/head_fail.json" <<'JSON'
{"files":[{"path":["/","src","lib.rs"],"traces":[{"line":1,"stats":{"Line":0}},{"line":2,"stats":{"Line":0}},{"line":3,"stats":{"Line":0}},{"line":4,"stats":{"Line":0}},{"line":5,"stats":{"Line":0}}]},{"path":["/","src","data","client.rs"],"traces":[{"line":10,"stats":{"Line":0}},{"line":11,"stats":{"Line":0}}]}]}
JSON
  local base_n ok_n fail_n
  base_n=$(uncovered_total "$tmp/base.json")
  ok_n=$(uncovered_total "$tmp/head_ok.json")
  fail_n=$(uncovered_total "$tmp/head_fail.json")
  [[ "$base_n" == "3" && "$ok_n" == "2" && "$fail_n" == "7" ]] \
    || { echo "self-test FAIL: totals base=$base_n ok=$ok_n fail=$fail_n" >&2; return 1; }
  (( ok_n <= base_n )) && (( fail_n > base_n )) \
    || { echo "self-test FAIL: decision gates" >&2; return 1; }
  python3 "$PY" summary "$tmp/head_fail.json" / 10 | grep -q "integration (I/O path)" \
    || { echo "self-test FAIL: I/O heuristic missing for data/client.rs" >&2; return 1; }
  echo "self-test OK (base=$base_n ok=$ok_n fail=$fail_n, heuristics verified)"
}

main() {
  case "${1:-}" in
    --self-test) self_test ;;
    --summary)   summarize_json "${2:?need tarpaulin JSON path}" ;;
    --base)      ratchet "${2:?need base ref}" ;;
    -h|--help)   sed -n '1,30p' "$0" ;;
    "")          ratchet "$COVERAGE_BASE_REF" ;;
    *)           echo "unknown arg: $1" >&2; exit 64 ;;
  esac
}
main "$@"
