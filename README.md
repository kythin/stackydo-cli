# todo

A context-aware CLI task manager with a TUI interface. Tasks are stored as markdown files with YAML frontmatter in `~/.todos/`.

## Features

- **TUI mode** — ratatui-based list+detail pane with filtering, sorting, and keyboard navigation
- **Headless CLI** — create, list, search, complete, and delete tasks from scripts and pipelines
- **Automatic context capture** — records git branch/commit, working directory, and project context on task creation
- **Task graph** — subtasks, dependencies (blocked-by, blocks, related-to)
- **`.todo-context` files** — define project-level context that gets attached to new tasks automatically
- **Session chaining** — tracks the last task ID created per shell session via `$TODO_LAST_ID`

## Install

```bash
cargo install --path .
```

## Quick Start

```bash
# Create a task (headless)
todo create --title "Fix auth bug" --tags "backend,urgent" --priority high -- The login endpoint returns 500 when the token expires

# Create with context pointing to a specific file and line
todo create --title "Review error handling" \
  --context-path src/api/handler.rs \
  --context-path-line 142 \
  --context-path-lookfor "unwrap\(\)" \
  -- Found several unwraps that should be proper error handling

# List tasks
todo list
todo list --status todo --sort priority
todo list --tag backend --limit 10

# Show task detail
todo show 01HQ        # prefix matching works

# Complete a task
todo complete 01HQ

# Search
todo search "auth"

# Launch TUI (default)
todo
```

## TUI Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` or arrows | Navigate task list |
| `c` | Complete selected task |
| `d` | Delete selected task |
| `s` | Cycle sort field (created → due → priority → modified) |
| `S` | Reverse sort order |
| `f` | Cycle status filter |
| `/` | Search mode |
| `Esc` | Clear search/filter |
| `r` | Reload tasks from disk |
| `q` | Quit |

## Task Storage

Each task is a markdown file at `~/.todos/<ULID>.md`:

```markdown
---
id: 01HQXYZ...
title: Fix auth bug
status: todo
priority: high
tags: [backend, urgent]
created: 2025-02-13T10:30:00Z
modified: 2025-02-13T10:30:00Z
context:
  working_dir: /home/user/project
  git_branch: main
  git_commit: a3f7d92
---

The login endpoint returns 500 when the token expires.
```

A manifest at `~/.todos/manifest.json` tracks tags, categories, and settings.

## Context Discovery

On task creation, `todo` automatically captures:

1. Current working directory
2. Git branch, remote URL, and HEAD commit (if in a repo)
3. Content from the nearest `.todo-context` file (walks up from CWD, falls back to `~/.todo-context`)
4. `$TODO_LAST_ID` — the ID of the previous task created in the same shell session

## License

MIT
