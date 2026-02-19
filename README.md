# stackydo

**One person's entire workload, in one place.**

Stackydo is a context-aware CLI task manager designed for individual engineers, leads, and makers who juggle work across many projects, teams, and responsibilities. Tasks are plain markdown files with YAML frontmatter — no database, no server, no vendor lock-in.

The core idea: your work doesn't live in one project. You might be debugging a production incident, reviewing a teammate's PR, prepping a conference talk, and planning next sprint — all in the same afternoon. Stackydo uses **stacks** to separate these workstreams while keeping everything in a single, searchable task store that's trivial for AI tools to read, enrich, and act on.

## Why This Exists

Most task managers are built for teams or for single projects. Stackydo is built for **you** — the individual who needs to:

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
- **`.stackydo-context` files** — define project-level context that gets attached to new tasks automatically
- **Session chaining** — tracks the last task ID created per shell session via `$STACKYDO_LAST_ID`
- **Configurable storage** — set `$STACKYDO_DIR` to relocate the task store (defaults to `~/.stackydo/`)

## Install

### Homebrew (macOS/Linux)

```bash
brew tap kythin/stackydo && brew install stackydo
```

### Shell (curl one-liner)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kythin/stackydo-cli/releases/latest/download/stackydo-installer.sh | sh
```

### PowerShell (Windows)

```powershell
powershell -c "irm https://github.com/kythin/stackydo-cli/releases/latest/download/stackydo-installer.ps1 | iex"
```

### From source

```bash
cargo install --git https://github.com/kythin/stackydo-cli
```

### Update

```bash
# Homebrew
brew upgrade stackydo

# Shell — re-run the curl installer, or use the built-in updater:
stackydo-update
```

All methods install two binaries: `stackydo` (CLI/TUI) and `stackydo-mcp` (MCP server).

## Quick Start

```bash
# Create a task (headless)
stackydo create --title "Fix auth bug" --tags "backend,urgent" --priority high --stack "work" -- The login endpoint returns 500 when the token expires

# Create with context pointing to a specific file and line
stackydo create --title "Review error handling" \
  --context-path src/api/handler.rs \
  --context-path-line 142 \
  --context-path-lookfor "unwrap\(\)" \
  -- Found several unwraps that should be proper error handling

# List tasks
stackydo list
stackydo list --status todo --sort priority
stackydo list --tag backend --limit 10
stackydo list --stack work              # filter by stack

# Show task detail
stackydo show 01HQ        # prefix matching works

# Complete a task
stackydo complete 01HQ

# Search
stackydo search "auth"

# Debug context capture (shows what would be recorded on create)
stackydo context

# Launch TUI (default when no subcommand given)
stackydo
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
stackydo create --title "Deploy v2" --stack "work"

# Filter tasks by stack
stackydo list --stack work
stackydo list --stack personal --status todo

# TUI create form includes a Stack field
```

The manifest tracks known stack names. Tasks without a stack are unstacked and won't appear in stack-filtered results.

## MCP Server (Claude Desktop / Claude Code)

Stackydo includes an MCP server that gives AI assistants full access to your task store. Build it once, then point your client at the binary.

### Install

Install stackydo using any method from the [Install](#install) section above. Both `stackydo` and `stackydo-mcp` are included.

### Claude Desktop

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS, `%APPDATA%\Claude\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "stackydo": {
      "command": "stackydo-mcp"
    }
  }
}
```

If you installed to a custom location or want to use a debug build, use the full path:

```json
{
  "mcpServers": {
    "stackydo": {
      "command": "/path/to/stackydo-mcp"
    }
  }
}
```

To use a non-default storage directory, add an `env` key:

```json
{
  "mcpServers": {
    "stackydo": {
      "command": "stackydo-mcp",
      "env": {
        "STACKYDO_DIR": "/path/to/your/tasks"
      }
    }
  }
}
```

Restart Claude Desktop after editing the config. The server communicates over stdio.

### Claude Code

Add to your project's `.claude/settings.local.json` or global settings:

```json
{
  "mcpServers": {
    "stackydo": {
      "command": "stackydo-mcp"
    }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `list_tasks` | List/filter tasks by status, tag, priority, stack, due date; sort and group |
| `get_task` | Get a single task by ID (prefix matching) |
| `create_task` | Create a task with title, priority, tags, stack, body, due date |
| `update_task` | Update fields, append timestamped notes |
| `complete_task` | Mark a task as done |
| `delete_task` | Permanently delete a task |
| `search_tasks` | Search title and body (case-insensitive) |
| `get_stats` | Summary statistics: totals, overdue count, breakdowns by status/stack/tag |
| `get_stacks` | All stacks with per-stack task counts and status breakdowns |

The server also exposes a `stackydo://guide` resource with a full agent guide, and prompt templates for triage, planning, and task extraction.

## Environment Variables

| Variable             | Description                                                              |
|----------------------|--------------------------------------------------------------------------|
| `STACKYDO_DIR`     | Override the task storage directory (default: `~/.stackydo/`)          |
| `STACKYDO_LAST_ID` | Set automatically by `stackydo create`; chains tasks in a shell session |

## Task Storage

Each task is a markdown file at `<STACKYDO_DIR>/<ULID>.md`:

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

A manifest at `<STACKYDO_DIR>/manifest.json` tracks tags, stacks, and settings.

## Context Discovery

On task creation, `stackydo` automatically captures:

1. Current working directory
2. Git branch, remote URL, and HEAD commit (if in a repo)
3. Content from the nearest `.stackydo-context` file (walks up from CWD, falls back to `~/.stackydo-context`)
4. `$STACKYDO_LAST_ID` — the ID of the previous task created in the same shell session

Use `stackydo context` to preview what would be captured without creating a task.

## Testing

```bash
cargo test                                     # unit tests
cargo build && bash tests/smoke_test.sh        # CLI smoke tests (76 assertions)
```

The smoke test uses `$STACKYDO_DIR` to write to a local `tests/.test-data/` directory — it never touches `~/.stackydo/`.

## Development Workspace

This repo includes a workspace-local task store at `.stackydo-workspace/` (gitignored) so that AI agents and contributors can use stackydo itself to track development work. A `.stackydo-context` file at the repo root provides project context for new tasks.

```bash
# Use the local workspace instead of ~/.stackydo/
export STACKYDO_DIR=.stackydo-workspace

stackydo create --title "Fix parser edge case" --stack dev -- Details here
stackydo list --stack dev
```

See `CLAUDE.md` for full agent dogfooding instructions.

## License

MIT
