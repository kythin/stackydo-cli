---
name: stackydo-iterative-implement
description: "Pull tasks from a stackydo stack, implement in parallel worktrees with subagents, review, merge, push"
allowed-tools:
  - Agent
  - Read
  - Glob
  - Grep
  - Bash
  - Edit
  - Write
  - mcp__stackydo-cargo__list_tasks
  - mcp__stackydo-cargo__create_task
  - mcp__stackydo-cargo__search_tasks
  - mcp__stackydo-cargo__update_task
  - mcp__stackydo-cargo__get_task
  - mcp__stackydo-cargo__complete_task
---

# Iterative Implement

Pull tasks from a stackydo stack, implement them in parallel using worktree subagents, review each result, then merge and push.

## Arguments

`$ARGUMENTS` is the required stack name. If empty, print an error: "Usage: /stackydo-iterative-implement <stack>" and stop.

## Step 1: Fetch and Group Tasks

1. Call `list_tasks` with the stack from `$ARGUMENTS`, sorted by priority (high first)
2. Read each task's full details with `get_task`
3. Group tasks into batches based on which files/modules they touch — tasks that modify overlapping files must be in the same batch (sequential), while tasks touching different areas can be parallelized
4. Present the grouping to the user:
   - Show each batch with its tasks, affected files, and estimated scope
   - Ask for approval before proceeding
   - The user may reorder, split, or merge batches

## Step 2: Implement Each Batch

For each batch, spawn subagents to work in parallel (one per task within the batch, but only if they touch non-overlapping files):

Each subagent must:
1. Update the task status to `in_progress` via `update_task`
2. Read and understand the relevant code thoroughly before making changes
3. Implement the fix or feature described in the task
4. Run `cargo test` and `cargo clippy` — both must pass
5. If the change affects CLI help text, TUI display, or docs, update those too
6. If the subagent discovers pre-existing issues in code it touches:
   - Search stackydo tasks (stack=audit) for existing reports
   - If not already tracked: create a new task with stack=audit
7. Commit with a descriptive message referencing the task ID

## Step 3: Review Each Completion

As each subagent completes, the orchestrator must:

1. Read the full diff of all changes
2. Review for:
   - **Correctness**: Does the change actually fix/implement what the task describes?
   - **Error handling**: No panicking unwraps, proper error propagation
   - **Test coverage**: Are there tests for the new/changed behavior?
   - **Style**: Consistent with existing code patterns
3. If the review passes:
   - Merge the changes into main
   - Run `cargo test` to verify no integration issues
   - Mark the stackydo task as `done` via `complete_task`
4. If the review fails:
   - Note the specific issues on the task via `update_task --note`
   - Either fix the issues directly or mark the task as `blocked`

## Step 4: Final Verification

After all batches are complete:

1. Run the full test suite: `cargo test` and `bash tests/smoke_test.sh`
2. If tests pass: `git push`
3. If tests fail: investigate, fix, and re-run before pushing
4. Print a summary: tasks completed, tasks blocked, any new audit tasks created
