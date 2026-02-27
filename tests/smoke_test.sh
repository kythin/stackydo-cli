#!/usr/bin/env bash
# smoke_test.sh — Scripted smoke tests for the stackydo CLI
#
# Usage:
#   cargo build && bash tests/smoke_test.sh
#
# Uses STACKYDO_DIR to isolate test data in tests/.test-data/ — never touches ~/.stackydo.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_DATA="$SCRIPT_DIR/.test-data"
export STACKYDO_DIR="$TEST_DATA"

TODO_BIN="$PROJECT_DIR/target/debug/stackydo"
PASS=0
FAIL=0
TESTS_RUN=0

# ── Helpers ──────────────────────────────────────────────────────────────

pass() {
    (( ++PASS ))
    (( ++TESTS_RUN ))
    echo "  ✓ $1"
}

fail() {
    (( ++FAIL ))
    (( ++TESTS_RUN ))
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
    echo "Using STACKYDO_DIR=$TEST_DATA"
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

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 15: JSON output (--json flag)
# ════════════════════════════════════════════════════════════════════════
section "Scenario 15: JSON output"

assert_json_valid() {
    local output="$1" label="$2"
    if echo "$output" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
        pass "$label"
    else
        fail "$label" "Output is not valid JSON"
    fi
}

# list --json
LIST_JSON=$($TODO_BIN list --json 2>&1)
assert_json_valid "$LIST_JSON" "list --json: valid JSON"
LIST_JSON_COUNT=$(echo "$LIST_JSON" | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null)
if [[ "$LIST_JSON_COUNT" -gt 0 ]]; then
    pass "list --json: returns array with items"
else
    fail "list --json: returns array with items" "Got $LIST_JSON_COUNT items"
fi

# show --json
SHOW_JSON=$($TODO_BIN show "$ID1" --json 2>&1)
assert_json_valid "$SHOW_JSON" "show --json: valid JSON"
assert_contains "$SHOW_JSON" '"Buy groceries"' "show --json: contains title"

# search --json
SEARCH_JSON=$($TODO_BIN search "groceries" --json 2>&1)
assert_json_valid "$SEARCH_JSON" "search --json: valid JSON"
assert_contains "$SEARCH_JSON" '"Buy groceries"' "search --json: contains matching title"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 16: --note on update
# ════════════════════════════════════════════════════════════════════════
section "Scenario 16: --note on update"

ID_NOTE=$($TODO_BIN create --title "Note test task" 2>&1)
$TODO_BIN update "$ID_NOTE" --note "First progress update" >/dev/null 2>&1
OUT_NOTE=$($TODO_BIN show "$ID_NOTE" 2>&1)
assert_contains "$OUT_NOTE" "First progress update" "note: text appears in body"
# Timestamp format: [YYYY-MM-DD HH:MM]
if echo "$OUT_NOTE" | grep -qF "[20"; then
    pass "note: timestamp prefix present"
else
    fail "note: timestamp prefix present" "Expected [20 in output"
fi

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 17: Due date filters
# ════════════════════════════════════════════════════════════════════════
section "Scenario 17: Due date filters"

# Create a task with a past due date (overdue)
ID_OVERDUE=$($TODO_BIN create --title "Overdue task" --due "2025-01-01" 2>&1)
# Create a task with a future due date
ID_FUTURE=$($TODO_BIN create --title "Future task" --due "2099-12-31" 2>&1)

# --overdue
OVERDUE_OUT=$($TODO_BIN list --overdue 2>&1)
assert_contains "$OVERDUE_OUT" "Overdue task" "list --overdue: finds overdue task"
assert_not_contains "$OVERDUE_OUT" "Future task" "list --overdue: excludes future task"

# --due-before
DUE_BEFORE=$($TODO_BIN list --due-before "2026-01-01" 2>&1)
assert_contains "$DUE_BEFORE" "Overdue task" "list --due-before: finds past due"

# --due-after
DUE_AFTER=$($TODO_BIN list --due-after "2090-01-01" 2>&1)
assert_contains "$DUE_AFTER" "Future task" "list --due-after: finds future task"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 18: Group-by stack
# ════════════════════════════════════════════════════════════════════════
section "Scenario 18: Group-by stack"

GROUP_OUT=$($TODO_BIN list --group-by stack 2>&1)
assert_contains "$GROUP_OUT" "[work]" "group-by stack: work header"
assert_contains "$GROUP_OUT" "[ops]" "group-by stack: ops header"
assert_contains "$GROUP_OUT" "total" "group-by stack: total footer"

GROUP_JSON=$($TODO_BIN list --group-by stack --json 2>&1)
assert_json_valid "$GROUP_JSON" "group-by stack --json: valid JSON"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 19: Stats command
# ════════════════════════════════════════════════════════════════════════
section "Scenario 19: Stats command"

STATS=$($TODO_BIN stats 2>&1)
assert_contains "$STATS" "Total tasks:" "stats: shows total"
assert_contains "$STATS" "By status:" "stats: shows by_status"
assert_contains "$STATS" "By stack:" "stats: shows by_stack"

STATS_JSON=$($TODO_BIN stats --json 2>&1)
assert_json_valid "$STATS_JSON" "stats --json: valid JSON"
assert_contains "$STATS_JSON" '"total"' "stats --json: has total field"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 20: Stacks command
# ════════════════════════════════════════════════════════════════════════
section "Scenario 20: Stacks command"

STACKS=$($TODO_BIN stacks 2>&1)
assert_contains "$STACKS" "work:" "stacks: work listed"
assert_contains "$STACKS" "ops:" "stacks: ops listed"

STACKS_JSON=$($TODO_BIN stacks --json 2>&1)
assert_json_valid "$STACKS_JSON" "stacks --json: valid JSON"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 21: Init command
# ════════════════════════════════════════════════════════════════════════
section "Scenario 21: Init command"

INIT_DIR=$(mktemp -d)
INIT_OUT=$($TODO_BIN init --yes --dir "$INIT_DIR/new-workspace" 2>&1)
assert_contains "$INIT_OUT" "initialized" "init --yes: reports initialized"
if [[ -f "$INIT_DIR/new-workspace/manifest.json" ]]; then
    pass "init: manifest.json created"
else
    fail "init: manifest.json created"
fi

# Init with --git
INIT_GIT_DIR=$(mktemp -d)
INIT_GIT_OUT=$($TODO_BIN init --yes --git --dir "$INIT_GIT_DIR/git-workspace" 2>&1)
if [[ -d "$INIT_GIT_DIR/git-workspace/.git" ]]; then
    pass "init --git: .git directory created"
else
    fail "init --git: .git directory created"
fi
rm -rf "$INIT_DIR" "$INIT_GIT_DIR"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 22: Import command
# ════════════════════════════════════════════════════════════════════════
section "Scenario 22: Import command"

IMPORT_OUT=$(echo '[{"title":"Imported task 1","priority":"high","tags":["import"],"stack":"imports"},{"title":"Imported task 2","body":"Some body"}]' | $TODO_BIN import 2>&1)
assert_contains "$IMPORT_OUT" "Imported 2 task" "import: reports 2 imported"

# Verify imported tasks exist
IMPORT_LIST=$($TODO_BIN list --stack imports 2>&1)
assert_contains "$IMPORT_LIST" "Imported task 1" "import: task 1 findable by stack"

IMPORT_SEARCH=$($TODO_BIN search "Imported task 2" 2>&1)
assert_contains "$IMPORT_SEARCH" "Imported task 2" "import: task 2 searchable"

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 23: list-workspaces command
# ════════════════════════════════════════════════════════════════════════
section "Scenario 23: list-workspaces command"

# list-workspaces discovers workspaces (at minimum, global default if it exists)
LW_OUT=$($TODO_BIN list-workspaces 2>&1)
# Just check it runs without error — actual workspaces depend on user's system
if [[ $? -eq 0 ]]; then
    pass "list-workspaces: runs without error"
else
    fail "list-workspaces: runs without error"
fi

# Alias 'lw' should also work
LW_ALIAS=$($TODO_BIN lw 2>&1)
if [[ $? -eq 0 ]]; then
    pass "list-workspaces: alias 'lw' works"
else
    fail "list-workspaces: alias 'lw' works"
fi

# JSON output
LW_JSON=$($TODO_BIN list-workspaces --json 2>&1)
if echo "$LW_JSON" | python3 -c "import sys,json; json.load(sys.stdin)" 2>/dev/null; then
    pass "list-workspaces --json: valid JSON"
else
    # Could be "No stackydo workspaces found." which is OK too
    if echo "$LW_JSON" | grep -q "No stackydo"; then
        pass "list-workspaces --json: no workspaces found (OK)"
    else
        fail "list-workspaces --json: valid JSON"
    fi
fi

# ════════════════════════════════════════════════════════════════════════
# SCENARIO 24: migrate command (non-interactive)
# ════════════════════════════════════════════════════════════════════════
section "Scenario 24: migrate command"

# Create two isolated workspace directories for migration testing
MIGRATE_SRC=$(mktemp -d)
MIGRATE_DST=$(mktemp -d)

# Seed source workspace with tasks
STACKYDO_DIR="$MIGRATE_SRC" $TODO_BIN create --title "Migrate task 1" --stack "work" >/dev/null 2>&1
STACKYDO_DIR="$MIGRATE_SRC" $TODO_BIN create --title "Migrate task 2" --stack "work" >/dev/null 2>&1
STACKYDO_DIR="$MIGRATE_SRC" $TODO_BIN create --title "Migrate task 3" --stack "personal" >/dev/null 2>&1

# Verify source has 3 tasks
SRC_COUNT=$(STACKYDO_DIR="$MIGRATE_SRC" $TODO_BIN list 2>&1)
assert_contains "$SRC_COUNT" "3 task" "migrate: source has 3 tasks"

# Test: missing required args
if $TODO_BIN migrate --source "$MIGRATE_SRC" --dest "$MIGRATE_DST" 2>/dev/null; then
    fail "migrate: requires --move or --copy" "succeeded without operation flag"
else
    pass "migrate: requires --move or --copy"
fi

# Test: dry-run copy by stack
DRY_OUT=$($TODO_BIN migrate --source "$MIGRATE_SRC" --dest "$MIGRATE_DST" --stack work --copy --dry-run 2>&1)
assert_contains "$DRY_OUT" "Dry run" "migrate --dry-run: shows dry run header"
assert_contains "$DRY_OUT" "Migrate task 1" "migrate --dry-run: lists task 1"
assert_contains "$DRY_OUT" "Migrate task 2" "migrate --dry-run: lists task 2"

# Verify destination is still empty after dry run
DST_COUNT_EMPTY=$(STACKYDO_DIR="$MIGRATE_DST" $TODO_BIN list 2>&1)
assert_contains "$DST_COUNT_EMPTY" "No tasks found" "migrate --dry-run: dest unchanged"

# Test: actual copy by stack
COPY_OUT=$($TODO_BIN migrate --source "$MIGRATE_SRC" --dest "$MIGRATE_DST" --stack work --copy 2>&1)
assert_contains "$COPY_OUT" "Copied 2 task" "migrate --copy: copied 2 tasks"

# Verify destination has 2 tasks
DST_AFTER_COPY=$(STACKYDO_DIR="$MIGRATE_DST" $TODO_BIN list 2>&1)
assert_contains "$DST_AFTER_COPY" "2 task" "migrate --copy: dest has 2 tasks"

# Verify source still has all 3 (copy preserves source)
SRC_AFTER_COPY=$(STACKYDO_DIR="$MIGRATE_SRC" $TODO_BIN list 2>&1)
assert_contains "$SRC_AFTER_COPY" "3 task" "migrate --copy: source unchanged"

# Test: move remaining task from different stack
MOVE_OUT=$($TODO_BIN migrate --source "$MIGRATE_SRC" --dest "$MIGRATE_DST" --stack personal --move 2>&1)
assert_contains "$MOVE_OUT" "Moved 1 task" "migrate --move: moved 1 task"

# Verify source lost the moved task
SRC_AFTER_MOVE=$(STACKYDO_DIR="$MIGRATE_SRC" $TODO_BIN list 2>&1)
assert_not_contains "$SRC_AFTER_MOVE" "Migrate task 3" "migrate --move: task removed from source"

# Verify destination gained it
DST_AFTER_MOVE=$(STACKYDO_DIR="$MIGRATE_DST" $TODO_BIN list 2>&1)
assert_contains "$DST_AFTER_MOVE" "3 task" "migrate --move: dest has 3 tasks"

# Test: conflict detection (copy same tasks again without --force)
CONFLICT_OUT=$($TODO_BIN migrate --source "$MIGRATE_SRC" --dest "$MIGRATE_DST" --stack work --copy 2>&1)
assert_contains "$CONFLICT_OUT" "conflict" "migrate: detects ID conflicts"

# Test: --force overwrites conflicts
FORCE_OUT=$($TODO_BIN migrate --source "$MIGRATE_SRC" --dest "$MIGRATE_DST" --stack work --copy --force 2>&1)
assert_contains "$FORCE_OUT" "Copied 2 task" "migrate --force: overwrites conflicts"

# Cleanup
rm -rf "$MIGRATE_SRC" "$MIGRATE_DST"

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
