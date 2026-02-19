#!/usr/bin/env bash
# scale_test.sh — Create 200+ tasks and validate list/search/filter/stats
#
# Usage:
#   cargo build && bash tests/scenarios/scale_test.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_DATA=$(mktemp -d)
export STACKYDO_DIR="$TEST_DATA"

TODO_BIN="$PROJECT_DIR/target/debug/stackydo"
PASS=0
FAIL=0
TESTS_RUN=0

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

assert_json_valid() {
    local output="$1" label="$2"
    if echo "$output" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        pass "$label"
    else
        fail "$label" "Output is not valid JSON"
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

if [[ ! -x "$TODO_BIN" ]]; then
    echo "Binary not found at $TODO_BIN — run 'cargo build' first."
    exit 1
fi

echo "Using STACKYDO_DIR=$TEST_DATA"

# ── Seed 200+ tasks ────────────────────────────────────────────────────

section "Creating 210 tasks across 7 stacks"

STACKS=("atlas" "nexus" "infra" "personal" "mobile" "backend" "frontend")
PRIORITIES=("critical" "high" "medium" "low")
TOTAL=210

START_TIME=$(date +%s)

for i in $(seq 1 $TOTAL); do
    stack_idx=$(( (i - 1) % ${#STACKS[@]} ))
    pri_idx=$(( (i - 1) % ${#PRIORITIES[@]} ))
    stack="${STACKS[$stack_idx]}"
    pri="${PRIORITIES[$pri_idx]}"
    $TODO_BIN create --title "Scale task $i" --stack "$stack" --priority "$pri" --tags "scale,batch$((i % 5))" >/dev/null 2>&1
done

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))
echo "  Created $TOTAL tasks in ${ELAPSED}s"

# Complete 50 tasks
for i in $(seq 1 50); do
    ID=$($TODO_BIN list --limit 1 --status todo 2>&1 | head -1 | awk '{print $2}')
    if [[ -n "$ID" ]]; then
        $TODO_BIN complete "$ID" >/dev/null 2>&1
    fi
done
echo "  Completed 50 tasks"

# ── Validate list ──────────────────────────────────────────────────────

section "Validating list operations"

START_TIME=$(date +%s)
ALL=$($TODO_BIN list 2>&1)
END_TIME=$(date +%s)
assert_contains "$ALL" "210 task" "list all: correct count"
echo "  list all: $((END_TIME - START_TIME))s"

# Filter by stack
ATLAS=$($TODO_BIN list --stack atlas 2>&1)
assert_contains "$ATLAS" "30 task" "list --stack atlas: 30 tasks"

# Filter by status
DONE=$($TODO_BIN list --status done 2>&1)
assert_contains "$DONE" "50 task" "list --status done: 50 tasks"

# JSON output
START_TIME=$(date +%s)
JSON_ALL=$($TODO_BIN list --json 2>&1)
END_TIME=$(date +%s)
assert_json_valid "$JSON_ALL" "list --json: valid JSON"
JSON_COUNT=$(echo "$JSON_ALL" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))")
if [[ "$JSON_COUNT" == "210" ]]; then
    pass "list --json: 210 items"
else
    fail "list --json: expected 210, got $JSON_COUNT"
fi
echo "  list --json: $((END_TIME - START_TIME))s"

# ── Validate search ───────────────────────────────────────────────────

section "Validating search"

START_TIME=$(date +%s)
SEARCH=$($TODO_BIN search "Scale task 1" 2>&1)
END_TIME=$(date +%s)
assert_contains "$SEARCH" "Scale task 1" "search: finds results"
echo "  search: $((END_TIME - START_TIME))s"

SEARCH_JSON=$($TODO_BIN search "Scale task 100" --json 2>&1)
assert_json_valid "$SEARCH_JSON" "search --json: valid JSON"

# ── Validate stats ─────────────────────────────────────────────────────

section "Validating stats"

START_TIME=$(date +%s)
STATS=$($TODO_BIN stats 2>&1)
END_TIME=$(date +%s)
assert_contains "$STATS" "Total tasks: 210" "stats: total count"
assert_contains "$STATS" "atlas:" "stats: atlas stack present"
echo "  stats: $((END_TIME - START_TIME))s"

STATS_JSON=$($TODO_BIN stats --json 2>&1)
assert_json_valid "$STATS_JSON" "stats --json: valid JSON"

# ── Validate stacks ───────────────────────────────────────────────────

section "Validating stacks"

STACKS_OUT=$($TODO_BIN stacks 2>&1)
for s in "${STACKS[@]}"; do
    assert_contains "$STACKS_OUT" "$s:" "stacks: $s listed"
done

STACKS_JSON=$($TODO_BIN stacks --json 2>&1)
assert_json_valid "$STACKS_JSON" "stacks --json: valid JSON"

# ── Validate group-by ─────────────────────────────────────────────────

section "Validating group-by"

GROUP=$($TODO_BIN list --group-by stack 2>&1)
assert_contains "$GROUP" "[atlas]" "group-by: atlas header"
assert_contains "$GROUP" "210 task" "group-by: total in footer"

GROUP_JSON=$($TODO_BIN list --group-by stack --json 2>&1)
assert_json_valid "$GROUP_JSON" "group-by --json: valid JSON"

# ── Summary ──────────────────────────────────────────────────────────

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Scale test: $PASS passed, $FAIL failed ($TESTS_RUN total)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
