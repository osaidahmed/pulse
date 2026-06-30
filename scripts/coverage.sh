#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

if [ -n "${REMOTE_RUN:-}" ] && [ -x "$ROOT_DIR/scripts/remote.sh" ]; then
  exec "$ROOT_DIR/scripts/remote.sh" "./scripts/$(basename "$0")" "$@"
fi

# ── usage ──────────────────────────────────────────────────────────
usage() {
  cat <<EOF
usage: $0 [options]

modes (pick one, default: --summary):
  --summary        text summary with per-file line coverage
  --html           generate HTML report and open in browser
  --json           output JSON coverage data
  --lcov           output LCOV format (for CI integration)

options:
  --fail-under N   fail if total coverage < N% (default: none)
  --no-open        skip auto-opening HTML report
  --ignore-tests   exclude test files from coverage metrics
  -h, --help       show this help

examples:
  $0                          text summary
  $0 --html                   HTML report, opens in browser
  $0 --summary --fail-under 70   fail if below 70%
  $0 --lcov > coverage.lcov   export for CI
EOF
  exit 0
}

# ── check dependencies ────────────────────────────────────────────
if ! command -v cargo-llvm-cov &>/dev/null; then
  echo "error: cargo-llvm-cov not installed"
  echo "install: cargo install cargo-llvm-cov"
  echo "also:    rustup component add llvm-tools-preview"
  exit 1
fi

if ! command -v cargo-nextest &>/dev/null; then
  echo "error: cargo-nextest not installed"
  echo "install: cargo install cargo-nextest"
  exit 1
fi

# ── parse arguments ────────────────────────────────────────────────
mode="summary"
fail_under=""
open_report=true
ignore_tests=false

while [ $# -gt 0 ]; do
  case "$1" in
  --summary) mode="summary" ;;
  --html) mode="html" ;;
  --json) mode="json" ;;
  --lcov) mode="lcov" ;;
  --no-open) open_report=false ;;
  --ignore-tests) ignore_tests=true ;;
  --fail-under)
    shift
    fail_under="$1"
    ;;
  -h | --help) usage ;;
  *)
    echo "error: unknown option '$1'" >&2
    echo "run '$0 --help' for usage" >&2
    exit 1
    ;;
  esac
  shift
done

# ── build flags ────────────────────────────────────────────────────
cd "$ROOT_DIR"

cov_flags=""

if [ "$ignore_tests" = true ]; then
  cov_flags="$cov_flags --ignore-filename-regex='tests/'"
fi

if [ -n "$fail_under" ]; then
  cov_flags="$cov_flags --fail-under-lines=$fail_under"
fi

# ── run coverage ──────────────────────────────────────────────────
case "$mode" in
summary)
  echo "running tests with coverage..."
  echo ""
  # shellcheck disable=SC2086
  cargo llvm-cov nextest --workspace $cov_flags 2>&1
  ;;

html)
  echo "generating HTML coverage report..."
  # shellcheck disable=SC2086
  cargo llvm-cov nextest --workspace --html $cov_flags 2>&1

  if [ "$open_report" = true ]; then
    echo ""
    report_dir="target/llvm-cov/html"
    case "$(uname -s)" in
    Darwin) open "$report_dir/index.html" ;;
    *) xdg-open "$report_dir/index.html" 2>/dev/null || true ;;
    esac
  fi

  echo ""
  echo "html report: target/llvm-cov/html/index.html"
  ;;

json)
  # shellcheck disable=SC2086
  cargo llvm-cov nextest --workspace --json $cov_flags 2>&1
  ;;

lcov)
  # shellcheck disable=SC2086
  cargo llvm-cov nextest --workspace --lcov $cov_flags 2>&1
  ;;
esac
