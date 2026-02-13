#!/usr/bin/env bash
# smoke_test.sh — Scripted smoke tests for the stackstodo CLI
#
# Usage:
#   cargo build && bash tests/smoke_test.sh
#
# Uses STACKSTODO_DIR to isolate test data in tests/.test-data/ — never touches ~/.stackstodo.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_DATA="$SCRIPT_DIR/.test-data"
export STACKSTODO_DIR="$TEST_DATA"

TODO_BIN="$PROJECT_DIR/target/debug/stackstodo"
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

assert_exit_code() {
    local expected="$1" label="$2"
    shift 2
    if "$@" >/dev/null 2>&1; then
        actual=0
    else
        actual=$?
    fi
    if [[ "$actual" -eq "$expected" ]]; then
        pass "$label"
    else
        fail "$label" "Expected exit code $expected, got $actual"
    fi
}

section() {
    echo ""
    echo "━━━ $1 ━━━"
}

# ── Setup / Teardown ────────────────────────────────────────────────────

setup() {
    rm -rf "$TEST_DATA"
    mkdir -p "$TEST_DATA"
    echo "Using STACKSTODO_DIR=$TEST_DATA"
}

teardown() {
    rm -rf "$TEST_DATA"
    echo "Cleaned up test data."
}

trap teardown EXIT

# ── Ensure binary exists ────────────────────────────────────────────────

if [[ ! -x "$TODO_BIN" ]]; then
    echo "Binary not found at $TODO_BIN — run 'cargo build' first."
    exit 1
fi

setup

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 1: Create + Show roundtrip
# ════════════════════════════════════════════════════════════════════════
section "Scenario 1: Create + Show roundtrip"

# Basic create with title only
ID1=$($TODO_BIN create --title "Buy groceries" 2>&1)
assert_contains "$ID1" "." "create returns a ULID"

# Show the task
OUT1=$($TODO_BIN show "$ID1" 2>&1)
assert_contains "$OUT1" "Buy groceries" "show displays title"
assert_contains "$OUT1" "todo" "show displays default status as todo"

# Create with all options (including --stack)
ID2=$($TODO_BIN create \
    --title "Deploy v2.0" \
    --priority high \
    --tags "deploy,release,backend" \
    --stack "work" \
    --due "2026-03-15 17:00" \
    -- This is the deployment checklist for version 2.0 2>&1)

OUT2=$($TODO_BIN show "$ID2" 2>&1)
assert_contains "$OUT2" "Deploy v2.0" "show: title with options"
assert_contains "$OUT2" "high" "show: priority"
assert_contains "$OUT2" "deploy" "show: tags (deploy)"
assert_contains "$OUT2" "release" "show: tags (release)"
assert_contains "$OUT2" "2026-03-15" "show: due date"
assert_contains "$OUT2" "deployment checklist" "show: body content"
assert_contains "$OUT2" "Stack:    work" "show: stack field"

# Create with body but no explicit title (should use first line of body)
ID3=$($TODO_BIN create -- Fix the login page redirect loop 2>&1)
OUT3=$($TODO_BIN show "$ID3" 2>&1)
assert_contains "$OUT3" "Fix the login page redirect loop" "auto-title from body"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 2: Seed diverse tasks for filter/sort testing
# ════════════════════════════════════════════════════════════════════════
section "Scenario 2: Seeding diverse task set"

# We already have 3 tasks. Create more with variety.
ID_CRIT=$($TODO_BIN create --title "Server on fire" --priority critical --tags "ops,urgent" --stack "ops" 2>&1)
ID_MED=$($TODO_BIN create --title "Refactor auth module" --priority medium --tags "backend,tech-debt" --stack "work" 2>&1)
ID_LOW=$($TODO_BIN create --title "Update README typos" --priority low --tags "docs" --stack "personal" 2>&1)
ID_DUE1=$($TODO_BIN create --title "Quarterly report" --due "2026-02-20" --tags "reports" --stack "work" 2>&1)
ID_DUE2=$($TODO_BIN create --title "Tax filing" --due "2026-04-15" --tags "finance" --stack "personal" 2>&1)

# Complete some tasks to test status filtering
$TODO_BIN complete "$ID3" >/dev/null 2>&1
$TODO_BIN complete "$ID_LOW" >/dev/null 2>&1

echo "  Seeded 8 tasks (2 completed, 6 todo)"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 3: List with filters
# ════════════════════════════════════════════════════════════════════════
section "Scenario 3: List with filters"

# All tasks
ALL=$($TODO_BIN list 2>&1)
assert_contains "$ALL" "8 task" "list: shows all 8 tasks"

# Filter by status=todo (should be 6)
TODO_ONLY=$($TODO_BIN list --status todo 2>&1)
assert_contains "$TODO_ONLY" "6 task" "list --status todo: 6 tasks"
assert_not_contains "$TODO_ONLY" "Update README" "list --status todo: excludes completed task"

# Filter by status=done (should be 2)
DONE_ONLY=$($TODO_BIN list --status done 2>&1)
assert_contains "$DONE_ONLY" "2 task" "list --status done: 2 tasks"

# Filter by tag
TAG_OPS=$($TODO_BIN list --tag ops 2>&1)
assert_contains "$TAG_OPS" "Server on fire" "list --tag ops: finds ops-tagged task"
assert_contains "$TAG_OPS" "1 task" "list --tag ops: exactly 1 result"

# Filter by priority
PRI_CRIT=$($TODO_BIN list --priority critical 2>&1)
assert_contains "$PRI_CRIT" "Server on fire" "list --priority critical: correct task"
assert_contains "$PRI_CRIT" "1 task" "list --priority critical: exactly 1"

# Combo filter: status + tag
BACKEND_TODO=$($TODO_BIN list --status todo --tag backend 2>&1)
assert_contains "$BACKEND_TODO" "Refactor auth" "list --status todo --tag backend: finds match"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 4: List sorting
# ════════════════════════════════════════════════════════════════════════
section "Scenario 4: List sorting"

# Sort by priority (Note: Option ordering puts None < Some, so tasks without
# priority sort first; among prioritized tasks, Critical < High < Medium < Low)
SORTED_PRI=$($TODO_BIN list --sort priority --priority critical 2>&1)
assert_contains "$SORTED_PRI" "Server on fire" "sort by priority: critical tasks listed"

# Sort by due date
SORTED_DUE=$($TODO_BIN list --sort due --status todo 2>&1)
# Tasks without due date sort differently from those with
assert_contains "$SORTED_DUE" "Quarterly report" "sort by due: tasks with due dates present"

# Limit
LIMITED=$($TODO_BIN list --limit 3 2>&1)
assert_contains "$LIMITED" "3 task" "list --limit 3: exactly 3 results"

# Reverse
# Verify reverse sort changes order
SORTED_CREATED=$($TODO_BIN list --sort created 2>&1)
SORTED_CREATED_REV=$($TODO_BIN list --sort created --reverse 2>&1)
FIRST_CREATED=$(echo "$SORTED_CREATED" | head -1)
FIRST_CREATED_REV=$(echo "$SORTED_CREATED_REV" | head -1)
if [[ "$FIRST_CREATED" != "$FIRST_CREATED_REV" ]]; then
    pass "sort --reverse: changes ordering"
else
    fail "sort --reverse: changes ordering" "First line was the same both ways"
fi

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 5: Search
# ════════════════════════════════════════════════════════════════════════
section "Scenario 5: Search"

# Search by title keyword
SEARCH1=$($TODO_BIN search "groceries" 2>&1)
assert_contains "$SEARCH1" "Buy groceries" "search 'groceries': finds title match"
assert_contains "$SEARCH1" "1 result" "search 'groceries': exactly 1"

# Search by body keyword
SEARCH2=$($TODO_BIN search "deployment checklist" 2>&1)
assert_contains "$SEARCH2" "Deploy v2.0" "search 'deployment checklist': finds body match"

# Search with no results
SEARCH3=$($TODO_BIN search "xyznonexistent" 2>&1)
assert_contains "$SEARCH3" "No tasks matching" "search miss: correct message"

# Case-insensitive search
SEARCH4=$($TODO_BIN search "SERVER ON FIRE" 2>&1)
assert_contains "$SEARCH4" "Server on fire" "search is case-insensitive"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 6: Complete + Delete lifecycle
# ════════════════════════════════════════════════════════════════════════
section "Scenario 6: Complete + Delete lifecycle"

# Create a throwaway task
ID_LIFE=$($TODO_BIN create --title "Lifecycle test task" 2>&1)

# Verify it starts as todo
OUT_LIFE=$($TODO_BIN show "$ID_LIFE" 2>&1)
assert_contains "$OUT_LIFE" "todo" "lifecycle: starts as todo"

# Complete it
$TODO_BIN complete "$ID_LIFE" >/dev/null 2>&1
OUT_LIFE2=$($TODO_BIN show "$ID_LIFE" 2>&1)
assert_contains "$OUT_LIFE2" "done" "lifecycle: status changed to done"

# Delete it
$TODO_BIN delete "$ID_LIFE" --force >/dev/null 2>&1

# Verify it's gone (show should fail)
if $TODO_BIN show "$ID_LIFE" >/dev/null 2>&1; then
    fail "lifecycle: task deleted" "show still found the task"
else
    pass "lifecycle: task deleted (show returns error)"
fi

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 7: Prefix ID resolution
# ════════════════════════════════════════════════════════════════════════
section "Scenario 7: Prefix ID resolution"

# Use first 10 chars of a known ID (longer prefix avoids ambiguity with
# ULIDs created in the same second sharing the timestamp portion)
PREFIX="${ID1:0:10}"
OUT_PREFIX=$($TODO_BIN show "$PREFIX" 2>&1)
assert_contains "$OUT_PREFIX" "Buy groceries" "prefix resolution: 10-char prefix works"

# Full ID still works
OUT_FULL=$($TODO_BIN show "$ID1" 2>&1)
assert_contains "$OUT_FULL" "Buy groceries" "prefix resolution: full ID works"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 8: Context command
# ════════════════════════════════════════════════════════════════════════
section "Scenario 8: Context command"

CTX=$($TODO_BIN context 2>&1)
assert_contains "$CTX" "Working dir" "context: shows working dir label"
assert_contains "$CTX" "Task store" "context: shows task store path"
assert_contains "$CTX" "Manifest" "context: shows manifest path"

# If we're in a git repo, should show branch
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    assert_contains "$CTX" "Git branch" "context: shows git branch (in git repo)"
fi

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 9: Edge cases
# ════════════════════════════════════════════════════════════════════════
section "Scenario 9: Edge cases"

# Empty body (title-only task)
ID_EMPTY=$($TODO_BIN create --title "No body task" 2>&1)
OUT_EMPTY=$($TODO_BIN show "$ID_EMPTY" 2>&1)
assert_contains "$OUT_EMPTY" "No body task" "edge: title-only task works"
assert_not_contains "$OUT_EMPTY" "--- Body ---" "edge: no body section when body is empty"

# Task with special characters in title
ID_SPECIAL=$($TODO_BIN create --title "Fix bug #42: 'quotes' & \"double quotes\"" 2>&1)
OUT_SPECIAL=$($TODO_BIN show "$ID_SPECIAL" 2>&1)
assert_contains "$OUT_SPECIAL" "Fix bug" "edge: special chars in title"

# Create with no title and no body (should default to "Untitled")
ID_UNTITLED=$($TODO_BIN create 2>&1)
OUT_UNTITLED=$($TODO_BIN show "$ID_UNTITLED" 2>&1)
assert_contains "$OUT_UNTITLED" "Untitled" "edge: no title/body defaults to Untitled"

# Task not found
if $TODO_BIN show "NONEXISTENT_ID_12345" >/dev/null 2>&1; then
    fail "edge: nonexistent task returns error" "show succeeded unexpectedly"
else
    pass "edge: nonexistent task returns error"
fi

# Delete nonexistent task
if $TODO_BIN delete "NONEXISTENT_ID_12345" --force >/dev/null 2>&1; then
    fail "edge: delete nonexistent returns error" "delete succeeded unexpectedly"
else
    pass "edge: delete nonexistent returns error"
fi

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 10: Stack feature
# ════════════════════════════════════════════════════════════════════════
section "Scenario 10: Stack feature"

# Filter by stack=work (should include Deploy v2.0, Refactor auth, Quarterly report)
STACK_WORK=$($TODO_BIN list --stack work 2>&1)
assert_contains "$STACK_WORK" "Deploy v2.0" "list --stack work: includes Deploy v2.0"
assert_contains "$STACK_WORK" "Refactor auth" "list --stack work: includes Refactor auth"
assert_contains "$STACK_WORK" "3 task" "list --stack work: exactly 3 tasks"

# Filter by stack=ops (should be just Server on fire)
STACK_OPS=$($TODO_BIN list --stack ops 2>&1)
assert_contains "$STACK_OPS" "Server on fire" "list --stack ops: correct task"
assert_contains "$STACK_OPS" "1 task" "list --stack ops: exactly 1 task"

# Filter by stack=personal (includes completed tasks)
STACK_PERSONAL=$($TODO_BIN list --stack personal 2>&1)
assert_contains "$STACK_PERSONAL" "2 task" "list --stack personal: 2 tasks (including done)"

# Combo: stack + status
STACK_WORK_TODO=$($TODO_BIN list --stack work --status todo 2>&1)
assert_contains "$STACK_WORK_TODO" "3 task" "list --stack work --status todo: 3 tasks"

# Task without a stack should not appear in stack-filtered results
assert_not_contains "$STACK_WORK" "Buy groceries" "list --stack work: excludes unstacked task"

# Show displays stack
OUT_STACK=$($TODO_BIN show "$ID_CRIT" 2>&1)
assert_contains "$OUT_STACK" "Stack:    ops" "show: displays stack for ops task"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 11: Update command
# ════════════════════════════════════════════════════════════════════════
section "Scenario 11: Update command"

# Create a task to update
ID_UPD=$($TODO_BIN create --title "Update me" --priority low --tags "old" --stack "temp" 2>&1)

# Update title
$TODO_BIN update "$ID_UPD" --title "Updated title" >/dev/null 2>&1
OUT_UPD=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_contains "$OUT_UPD" "Updated title" "update: title changed"

# Update status
$TODO_BIN update "$ID_UPD" --status in_progress >/dev/null 2>&1
OUT_UPD2=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_contains "$OUT_UPD2" "in_progress" "update: status changed to in_progress"

# Update priority
$TODO_BIN update "$ID_UPD" --priority critical >/dev/null 2>&1
OUT_UPD3=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_contains "$OUT_UPD3" "critical" "update: priority changed"

# Clear priority with "none"
$TODO_BIN update "$ID_UPD" --priority none >/dev/null 2>&1
OUT_UPD4=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_not_contains "$OUT_UPD4" "Priority:" "update: priority cleared with none"

# Update tags
$TODO_BIN update "$ID_UPD" --tags "new,updated" >/dev/null 2>&1
OUT_UPD5=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_contains "$OUT_UPD5" "new" "update: tags replaced"
assert_not_contains "$OUT_UPD5" "old" "update: old tags removed"

# Update stack
$TODO_BIN update "$ID_UPD" --stack "work" >/dev/null 2>&1
OUT_UPD6=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_contains "$OUT_UPD6" "Stack:    work" "update: stack changed"

# Clear stack
$TODO_BIN update "$ID_UPD" --stack "" >/dev/null 2>&1
OUT_UPD7=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_not_contains "$OUT_UPD7" "Stack:" "update: stack cleared"

# Update due date
$TODO_BIN update "$ID_UPD" --due "2026-12-25" >/dev/null 2>&1
OUT_UPD8=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_contains "$OUT_UPD8" "2026-12-25" "update: due date set"

# Clear due date
$TODO_BIN update "$ID_UPD" --due "" >/dev/null 2>&1
OUT_UPD9=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_not_contains "$OUT_UPD9" "Due:" "update: due date cleared"

# Append body text
$TODO_BIN update "$ID_UPD" -- some extra body text >/dev/null 2>&1
OUT_UPD10=$($TODO_BIN show "$ID_UPD" 2>&1)
assert_contains "$OUT_UPD10" "some extra body text" "update: body text appended"

# No changes prints message
NO_CHANGE=$($TODO_BIN update "$ID_UPD" 2>&1)
assert_contains "$NO_CHANGE" "No changes specified" "update: no-op message"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 12: Dependencies
# ════════════════════════════════════════════════════════════════════════
section "Scenario 12: Dependencies"

ID_DEP_A=$($TODO_BIN create --title "Dep task A" 2>&1)
ID_DEP_B=$($TODO_BIN create --title "Dep task B" 2>&1)

# Add blocked-by via update
$TODO_BIN update "$ID_DEP_B" --blocked-by "$ID_DEP_A" >/dev/null 2>&1
OUT_DEP=$($TODO_BIN show "$ID_DEP_B" 2>&1)
assert_contains "$OUT_DEP" "BlockedBy" "update: blocked-by dependency added"
assert_contains "$OUT_DEP" "${ID_DEP_A:0:10}" "update: references correct dep task"

# Create with dependency flags
ID_DEP_C=$($TODO_BIN create --title "Dep task C" --blocked-by "$ID_DEP_A" 2>&1)
OUT_DEP_C=$($TODO_BIN show "$ID_DEP_C" 2>&1)
assert_contains "$OUT_DEP_C" "BlockedBy" "create: blocked-by dependency wired at creation"

# Create with parent
ID_PARENT=$($TODO_BIN create --title "Parent task" 2>&1)
ID_CHILD=$($TODO_BIN create --title "Child task" --parent "$ID_PARENT" 2>&1)
OUT_CHILD=$($TODO_BIN show "$ID_CHILD" 2>&1)
OUT_PARENT=$($TODO_BIN show "$ID_PARENT" 2>&1)
assert_contains "$OUT_PARENT" "${ID_CHILD:0:10}" "create: parent has child in subtask_ids"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 13: Enriched search output
# ════════════════════════════════════════════════════════════════════════
section "Scenario 13: Enriched search output"

# Create a task with priority and tags to verify search shows them
ID_RICH=$($TODO_BIN create --title "Rich search item" --priority high --tags "findme" --stack "searchtest" 2>&1)
SEARCH_RICH=$($TODO_BIN search "Rich search item" 2>&1)
assert_contains "$SEARCH_RICH" "[high]" "search: shows priority in output"
assert_contains "$SEARCH_RICH" "#findme" "search: shows tags in output"
assert_contains "$SEARCH_RICH" "@searchtest" "search: shows stack in output"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 14: Bulk operations
# ════════════════════════════════════════════════════════════════════════
section "Scenario 14: Bulk operations"

# Create tasks in a dedicated stack for bulk testing
ID_BULK1=$($TODO_BIN create --title "Bulk task 1" --stack "bulktest" 2>&1)
ID_BULK2=$($TODO_BIN create --title "Bulk task 2" --stack "bulktest" 2>&1)
ID_BULK3=$($TODO_BIN create --title "Bulk task 3" --stack "bulktest" 2>&1)

# Bulk complete requires --all
if $TODO_BIN complete --stack "bulktest" >/dev/null 2>&1; then
    fail "bulk complete: requires --all flag" "succeeded without --all"
else
    pass "bulk complete: requires --all flag"
fi

# Bulk complete with --all and filter
$TODO_BIN complete --stack "bulktest" --all >/dev/null 2>&1
BULK_LIST=$($TODO_BIN list --stack "bulktest" --status done 2>&1)
assert_contains "$BULK_LIST" "3 task" "bulk complete: all 3 tasks completed"

# Bulk delete requires --force --all
if $TODO_BIN delete --stack "bulktest" --all >/dev/null 2>&1; then
    fail "bulk delete: requires --force flag" "succeeded without --force"
else
    pass "bulk delete: requires --force flag"
fi

# Bulk delete with --force --all
$TODO_BIN delete --stack "bulktest" --force --all >/dev/null 2>&1
BULK_AFTER=$($TODO_BIN list --stack "bulktest" 2>&1)
assert_contains "$BULK_AFTER" "No tasks found" "bulk delete: all tasks deleted"

# ════════════════════════════════════════════════════════════════════════
# Summary
# ════════════════════════════════════════════════════════════════════════

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Results: $PASS passed, $FAIL failed ($TESTS_RUN total)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [[ "$FAIL" -gt 0 ]]; then
    exit 1
fi
