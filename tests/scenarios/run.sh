#!/usr/bin/env bash
# run.sh — Reusable scenario runner for stackydo CLI testing
#
# Usage:
#   bash tests/scenarios/run.sh [scenario_name]
#
# Creates a temp STACKYDO_DIR, runs commands, tracks IDs, asserts on output.
# Designed for scripted scenario validation.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_DATA=$(mktemp -d)
export STACKYDO_DIR="$TEST_DATA"

TODO_BIN="$PROJECT_DIR/target/debug/stackydo"
PASS=0
FAIL=0
TESTS_RUN=0

# ── Helpers ──────────────────────────────────────────────────────────────

pass() {
    ((PASS++))
    ((TESTS_RUN++))
    echo "  ✓ $1"
}

fail() {
    ((FAIL++))
    ((TESTS_RUN++))
    echo "  ✗ $1"
    if [[ -n "${2:-}" ]]; then
        echo "    → $2"
    fi
}

assert_contains() {
    local output="$1" expected="$2" label="$3"
    if echo "$output" | grep -qi -- "$expected"; then
        pass "$label"
    else
        fail "$label" "Expected to find '$expected' in output"
    fi
}

assert_not_contains() {
    local output="$1" unexpected="$2" label="$3"
    if echo "$output" | grep -qi -- "$unexpected"; then
        fail "$label" "Did NOT expect to find '$unexpected' in output"
    else
        pass "$label"
    fi
}

assert_json_valid() {
    local output="$1" label="$2"
    if echo "$output" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        pass "$label"
    else
        fail "$label" "Output is not valid JSON"
    fi
}

assert_json_count() {
    local output="$1" expected="$2" label="$3"
    local actual
    actual=$(echo "$output" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null || echo "-1")
    if [[ "$actual" == "$expected" ]]; then
        pass "$label"
    else
        fail "$label" "Expected $expected items, got $actual"
    fi
}

section() {
    echo ""
    echo "━━━ $1 ━━━"
}

teardown() {
    rm -rf "$TEST_DATA"
    echo "Cleaned up test data at $TEST_DATA"
}
trap teardown EXIT

# ── Ensure binary exists ────────────────────────────────────────────────

if [[ ! -x "$TODO_BIN" ]]; then
    echo "Binary not found at $TODO_BIN — run 'cargo build' first."
    exit 1
fi

echo "Using STACKYDO_DIR=$TEST_DATA"

# ── Seed data ───────────────────────────────────────────────────────────

section "Seeding scenario data"

# Create tasks across multiple stacks
ID_W1=$($TODO_BIN create --title "Work task 1" --stack "work" --priority medium --tags "backend" 2>&1)
ID_W2=$($TODO_BIN create --title "Work task 2" --stack "work" --priority medium --tags "backend" 2>&1)
ID_W3=$($TODO_BIN create --title "Work task 3" --stack "work" --priority medium --tags "backend" 2>&1)
ID_W4=$($TODO_BIN create --title "Work task 4" --stack "work" --priority medium --tags "backend" 2>&1)
ID_W5=$($TODO_BIN create --title "Work task 5" --stack "work" --priority medium --tags "backend" 2>&1)
ID_O1=$($TODO_BIN create --title "Ops task 1" --stack "ops" --priority high --tags "infra" 2>&1)
ID_O2=$($TODO_BIN create --title "Ops task 2" --stack "ops" --priority high --tags "infra" 2>&1)
ID_O3=$($TODO_BIN create --title "Ops task 3" --stack "ops" --priority high --tags "infra" 2>&1)
ID_P1=$($TODO_BIN create --title "Personal task 1" --stack "personal" 2>&1)
ID_P2=$($TODO_BIN create --title "Personal task 2" --stack "personal" 2>&1)

# Complete some
$TODO_BIN complete "$ID_W1" >/dev/null 2>&1
$TODO_BIN complete "$ID_O1" >/dev/null 2>&1

# Add due dates (past = overdue, future = upcoming)
$TODO_BIN update "$ID_W2" --due "2025-01-01" >/dev/null 2>&1
$TODO_BIN update "$ID_W3" --due "2099-12-31" >/dev/null 2>&1

echo "  Seeded 10 tasks across 3 stacks (2 completed, 1 overdue)"

# ── Test: stats command ─────────────────────────────────────────────────

section "Stats command"

STATS=$($TODO_BIN stats 2>&1)
assert_contains "$STATS" "Total tasks: 10" "stats: total count"
assert_contains "$STATS" "Overdue: 1" "stats: overdue count"
assert_contains "$STATS" "work:" "stats: work stack present"
assert_contains "$STATS" "ops:" "stats: ops stack present"

STATS_JSON=$($TODO_BIN stats --json 2>&1)
assert_json_valid "$STATS_JSON" "stats --json: valid JSON"
assert_contains "$STATS_JSON" '"total": 10' "stats --json: total field"

# ── Test: stacks command ────────────────────────────────────────────────

section "Stacks command"

STACKS=$($TODO_BIN stacks 2>&1)
assert_contains "$STACKS" "work:" "stacks: work listed"
assert_contains "$STACKS" "ops:" "stacks: ops listed"
assert_contains "$STACKS" "personal:" "stacks: personal listed"

STACKS_JSON=$($TODO_BIN stacks --json 2>&1)
assert_json_valid "$STACKS_JSON" "stacks --json: valid JSON"

# ── Test: overdue filter ────────────────────────────────────────────────

section "Due date filters"

OVERDUE=$($TODO_BIN list --overdue 2>&1)
assert_contains "$OVERDUE" "Work task 2" "list --overdue: finds overdue task"
assert_contains "$OVERDUE" "1 task" "list --overdue: exactly 1 overdue"

# ── Test: group-by ──────────────────────────────────────────────────────

section "Group-by"

GROUP=$($TODO_BIN list --group-by stack 2>&1)
assert_contains "$GROUP" "[work]" "group-by stack: work header"
assert_contains "$GROUP" "[ops]" "group-by stack: ops header"

GROUP_JSON=$($TODO_BIN list --group-by stack --json 2>&1)
assert_json_valid "$GROUP_JSON" "group-by stack --json: valid JSON"

# ── Summary ─────────────────────────────────────────────────────────────

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Scenario results: $PASS passed, $FAIL failed ($TESTS_RUN total)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
