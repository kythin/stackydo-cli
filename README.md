# stackstodo

**One person's entire workload, in one place.**

Stackstodo is a context-aware CLI task manager designed for individual engineers, leads, and makers who juggle work across many projects, teams, and responsibilities. Tasks are plain markdown files with YAML frontmatter — no database, no server, no vendor lock-in.

The core idea: your work doesn't live in one project. You might be debugging a production incident, reviewing a teammate's PR, prepping a conference talk, and planning next sprint — all in the same afternoon. Stackstodo uses **stacks** to separate these workstreams while keeping everything in a single, searchable task store that's trivial for AI tools to read, enrich, and act on.

## Why This Exists

Most task managers are built for teams or for single projects. Stackstodo is built for **you** — the individual who needs to:

- Track work across 3-5 projects, team duties, and personal goals simultaneously
- Create tasks from wherever you are (terminal, editor, scripts, AI agents)
- Search across everything at once ("what was that security thing last week?")
- Let AI tools triage, summarize, and report on your workload
- Own your data as plain files you can grep, version, and back up

The storage format is intentionally simple: one markdown file per task, human-readable, git-friendly, and easy for any tool to parse.

## Features

- **TUI mode** — ratatui-based list+detail pane with filtering, sorting, keyboard navigation, task creation, and settings
- **Headless CLI** — create, list, search, update, complete, delete from scripts and pipelines
- **Stacks** — organize tasks into named workstreams (e.g. "atlas", "leadership", "bugs", "personal")
- **Automatic context capture** — records git branch/commit, working directory, and project context on task creation
- **Task graph** — subtasks, dependencies (blocked-by, blocks, related-to)
- **AI-friendly storage** — plain markdown+YAML files that any LLM or script can read and write
- **`.stackstodo-context` files** — define project-level context that gets attached to new tasks automatically
- **Session chaining** — tracks the last task ID created per shell session via `$STACKSTODO_LAST_ID`
- **Configurable storage** — set `$STACKSTODO_DIR` to relocate the task store (defaults to `~/.stackstodo/`)

## Install

```bash
cargo install --path .
```

This installs two binaries: `stackstodo` (primary) and `todo` (alias).

## Quick Start

```bash
# Create a task (headless)
stackstodo create --title "Fix auth bug" --tags "backend,urgent" --priority high --stack "work" -- The login endpoint returns 500 when the token expires

# Create with context pointing to a specific file and line
stackstodo create --title "Review error handling" \
  --context-path src/api/handler.rs \
  --context-path-line 142 \
  --context-path-lookfor "unwrap\(\)" \
  -- Found several unwraps that should be proper error handling

# List tasks
stackstodo list
stackstodo list --status todo --sort priority
stackstodo list --tag backend --limit 10
stackstodo list --stack work              # filter by stack

# Show task detail
stackstodo show 01HQ        # prefix matching works

# Complete a task
stackstodo complete 01HQ

# Search
stackstodo search "auth"

# Debug context capture (shows what would be recorded on create)
stackstodo context

# Launch TUI (default when no subcommand given)
stackstodo
```

## TUI Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate task list |
| `g`/`G` | Jump to first/last task |
| `c` | Complete selected task |
| `d` | Delete selected task |
| `n` | Create new task (opens form) |
| `s` | Cycle sort field (created -> due -> priority -> modified) |
| `S` | Reverse sort order |
| `f` | Cycle status filter |
| `/` | Search mode |
| `,` | Open settings |
| `r` | Reload tasks from disk |
| `Esc` | Clear search/filter |
| `q` | Quit |

### Create Task Form (`n`)

| Key | Action |
|-----|--------|
| `Tab`/`Shift+Tab` | Next/previous field |
| Any key (on Priority) | Cycle priority (low -> medium -> high -> critical -> none) |
| `Backspace` | Delete character / clear priority |
| `Enter` | Submit and create the task |
| `Esc` | Cancel |

Fields: Title, Priority, Tags, Stack, Body

### Settings Screen (`,`)

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate settings |
| `Enter`/`Space` | Toggle/cycle the selected setting |
| `s` | Save settings to manifest |
| `Esc` | Back to main screen |

## Stacks

A task can belong to one **stack** — a named group like "work", "personal", or "sprint-12". Think of stacks as physical piles of tasks rather than flat database categories.

```bash
# Create a task on a stack
stackstodo create --title "Deploy v2" --stack "work"

# Filter tasks by stack
stackstodo list --stack work
stackstodo list --stack personal --status todo

# TUI create form includes a Stack field
```

The manifest tracks known stack names. Tasks without a stack are unstacked and won't appear in stack-filtered results.

## Environment Variables

| Variable             | Description                                                              |
|----------------------|--------------------------------------------------------------------------|
| `STACKSTODO_DIR`     | Override the task storage directory (default: `~/.stackstodo/`)          |
| `STACKSTODO_LAST_ID` | Set automatically by `stackstodo create`; chains tasks in a shell session |

## Task Storage

Each task is a markdown file at `<STACKSTODO_DIR>/<ULID>.md`:

```markdown
---
id: 01HQXYZ...
title: Fix auth bug
status: todo
priority: high
tags: [backend, urgent]
stack: work
created: 2025-02-13T10:30:00Z
modified: 2025-02-13T10:30:00Z
context:
  working_dir: /home/user/project
  git_branch: main
  git_commit: a3f7d92
---

The login endpoint returns 500 when the token expires.
```

A manifest at `<STACKSTODO_DIR>/manifest.json` tracks tags, stacks, and settings.

## Context Discovery

On task creation, `stackstodo` automatically captures:

1. Current working directory
2. Git branch, remote URL, and HEAD commit (if in a repo)
3. Content from the nearest `.stackstodo-context` file (walks up from CWD, falls back to `~/.stackstodo-context`)
4. `$STACKSTODO_LAST_ID` — the ID of the previous task created in the same shell session

Use `stackstodo context` to preview what would be captured without creating a task.

## Testing

```bash
cargo test                                     # unit tests
cargo build && bash tests/smoke_test.sh        # CLI smoke tests (76 assertions)
```

The smoke test uses `$STACKSTODO_DIR` to write to a local `tests/.test-data/` directory — it never touches `~/.stackstodo/`.

## License

MIT
