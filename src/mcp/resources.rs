use rmcp::model::{Annotated, RawResource, Resource};

pub const GUIDE_CONTENT: &str = r#"# Stackstodo Agent Guide

## What is Stackstodo?

Stackstodo is a personal workload manager for one individual across all projects. Tasks are stored as flat markdown files with YAML frontmatter in `~/.stackstodo/` (overridable via `$STACKSTODO_DIR`). No database — just files.

## Core Concepts

### Tasks
- Each task has a **ULID** (time-sortable unique ID)
- Stored as `<ULID>.md` with YAML frontmatter + freeform markdown body
- Fields: title, status, priority, tags, stack, due date, context, dependencies

### Statuses
- **todo** — not started
- **in_progress** — actively being worked on
- **done** — completed
- **blocked** — waiting on something
- **cancelled** — abandoned

### Priorities
- **critical** — drop everything
- **high** — do soon
- **medium** — normal (default when unset)
- **low** — backlog / someday

### Stacks
One stack per task. Stacks are workstream organizers — think "project" or "area of responsibility." Examples: `work`, `personal`, `side-project`, `home`.

Use stacks to separate workstreams, not tags. Tags are for cross-cutting concerns (e.g., `bug`, `meeting`, `idea`).

### Tags
Many tags per task. Comma-separated. Good for filtering across stacks.

### Dependencies
- **blocked_by** — this task can't start until another completes
- **blocks** — another task is waiting on this one
- **related_to** — loose association

### Context
Auto-captured at creation: git branch, remote, commit, working directory. Also discovers `.stackstodo-context` files by walking up from CWD.

## When to Create Tasks

### Coding
Bugs, features, refactors, tech debt. Use stacks per project. Context captures git branch/file automatically.

### Research
Reading lists, investigation threads. Use `note` field to append findings over time with timestamps.

### Day-to-day
Meetings, follow-ups, errands. Personal stack, due dates for deadlines.

### Backlog / Ideas
Low-priority captures for later review. Dedicated `ideas` stack or personal stack.

## How to Use Efficiently

1. **Start with `get_stats`** for situational awareness — total tasks, overdue count, breakdown by stack/status
2. **Search before creating** to avoid duplicates
3. **Use stacks to separate workstreams**, not tags — stacks are the primary organizer
4. **Use `note` for incremental progress** — appends timestamped entries to the body
5. **Use `list_tasks` with filters** rather than loading everything — filter by status, stack, tag, priority, overdue, due_before/after
6. **Group by stack** (`group_by: "stack"`) to see the full picture
7. **Use the `overdue` filter** to surface urgent items
8. **Prefix matching** works for task IDs — you don't need the full ULID, just enough to be unique

## Tool Reference

| Tool | Purpose |
|------|---------|
| `list_tasks` | List/filter/sort/group tasks |
| `get_task` | Get full task details by ID |
| `create_task` | Create a new task |
| `update_task` | Modify task fields, append notes |
| `complete_task` | Mark a task as done |
| `delete_task` | Permanently delete a task |
| `search_tasks` | Full-text search across titles and bodies |
| `get_stats` | Summary statistics |
| `get_stacks` | Stack listing with status counts |

## Prompts

| Prompt | Purpose |
|--------|---------|
| `triage` | Review open tasks, suggest priority/status changes |
| `plan_work` | Break a goal into actionable tasks |
| `daily_standup` | Summarize recent activity and blockers |
| `capture_todos` | Extract actionable items from text |
"#;

pub fn guide_resource() -> Resource {
    Annotated::new(
        RawResource {
            uri: "stackstodo://guide".into(),
            name: "Stackstodo Agent Guide".into(),
            title: None,
            description: Some(
                "Comprehensive guide for AI agents on how to use stackstodo effectively".into(),
            ),
            mime_type: Some("text/markdown".into()),
            size: None,
            icons: None,
            meta: None,
        },
        None,
    )
}
