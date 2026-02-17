#!/usr/bin/env bash
# test_all.sh — Run all test suites for stackstodo
#
# Usage:
#   bash tests/test_all.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
FAILED=0

section() {
    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "  $1"
    echo "════════════════════════════════════════════════════════════"
}

run_suite() {
    local name="$1"
    shift
    section "$name"
    if "$@"; then
        echo "  ✓ $name passed"
    else
        echo "  ✗ $name FAILED"
        ((FAILED++))
    fi
}

cd "$PROJECT_DIR"

run_suite "Clippy"          cargo clippy -- -D warnings
run_suite "Unit tests"      cargo test
run_suite "Build"           cargo build
run_suite "Smoke tests"     bash tests/smoke_test.sh
run_suite "Scenario test"   bash tests/scenarios/run.sh
run_suite "Scale test"      bash tests/scenarios/scale_test.sh

echo ""
echo "════════════════════════════════════════════════════════════"
if [[ "$FAILED" -eq 0 ]]; then
    echo "  All suites passed."
else
    echo "  $FAILED suite(s) FAILED."
    exit 1
fi
echo "════════════════════════════════════════════════════════════"
