# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`stackydo` is a context-aware CLI task manager written in Rust. Tasks are stored as individual markdown files with YAML frontmatter in `~/.stackydo/` (overridable via `$STACKYDO_DIR`). It supports a ratatui-based TUI (default), headless CLI commands, and an MCP server. Binaries: `stackydo` (primary/TUI+CLI), `stackydo-mcp` (MCP server over stdio).

## Build & Run

```bash
cargo build
cargo run              # launches TUI
cargo run -- create --title "My task" --stack "work" -- body content here
cargo run -- list
cargo run -- list --stack work
cargo run -- search "query"
cargo run -- context   # debug: show what context would be captured
cargo clippy
```

## Testing

```bash
cargo test                                # unit tests (in-module #[cfg(test)])
cargo test test_name                      # run a single test by name
cargo build && bash tests/smoke_test.sh   # CLI smoke tests
bash tests/test_all.sh                    # full suite: clippy + unit + build + smoke + scenario + scale
```

The smoke test sets `STACKYDO_DIR=tests/.test-data/` so it never touches `~/.stackydo/`. Unit tests use `tempfile` crate for filesystem isolation.

## Architecture

- **`src/main.rs`** — CLI entrypoint: parses clap args, dispatches to command handlers or launches TUI when no subcommand given
- **`src/mcp_bin.rs`** — MCP server entrypoint: runs `StackydoMcp` over stdio transport using rmcp
- **`src/model/`** — Core types: `Task`, `TaskFrontmatter`, `Manifest`, `ContextInfo`, `Stage` enum, `WorkflowConfig`, priority/dependency enums
- **`src/storage/`** — Filesystem persistence: markdown+YAML task files (`TaskStore`), JSON manifest (`ManifestStore`), path resolution (`TodoPaths`)
- **`src/context/`** — Auto-capture: git info via git2 (`git_context`), `.stackydo-context` file discovery (`dir_context`), combined capture (`todo_context`)
- **`src/cli/`** — clap derive argument definitions (`args.rs`)
- **`src/commands/`** — Headless command implementations: create, list, show, update, complete, delete, search, context, stats, stacks, init, import, body_edit (sed parser), migrate
- **`src/tui/`** — ratatui TUI: app state (`app.rs`), 40/60 split layout (`ui.rs`), keybinding handler (`handler.rs`), widgets (`widgets/`)
- **`src/mcp/`** — MCP server: tool definitions (`tools.rs`), prompt templates (`prompts.rs`), resource definitions (`resources.rs`)
- **`src/error.rs`** — `TodoError` enum using thiserror

## Key Design Decisions

- Tasks are flat files: `<STACKYDO_DIR>/<ULID>.md` — no database
- `$STACKYDO_DIR` env var overrides the default `~/.stackydo/` storage root; all path resolution goes through `TodoPaths::root()`
- ULIDs for IDs (time-sortable, unique); files named `<ULID>.md`
- **Short IDs**: Tasks get a human-friendly `short_id` (SD1, SD2, …) stored in frontmatter. Counter tracked in `manifest.next_short_id`, per-workspace, never recycled. All commands accept short IDs, ULIDs, or ULID prefixes. `display_id()` helper in `util.rs` shows short_id when available, falls back to truncated ULID
- All dates stored as UTC, displayed in local timezone
- YAML frontmatter is managed by the tool; body content is freeform markdown
- `.stackydo-context` files discovered by walking up from CWD, fallback `~/.stackydo-context`
- `$STACKYDO_LAST_ID` env var chains tasks created in the same shell session
- Task graph: subtasks (parent_id/subtask_ids) + dependencies (blocked_by, blocks, related_to)
- **Stacks**: each task has an optional `stack: Option<String>` field (one stack per task); manifest tracks known stack names
- **Stage/Status workflow**: `TaskFrontmatter.status` is a `String` (not an enum). `Stage` is a fixed enum (Backlog, Active, Archive) computed from status via `WorkflowConfig`. Default statuses: todo/on_hold (Backlog), in_progress/blocked/in_review (Active), done/cancelled (Archive). Archive-stage tasks are hidden from list/search by default. Delete is always a file operation — no soft-delete. `WorkflowConfig` stored in manifest with `#[serde(default)]` for backward compat.
- Storage layer uses concrete types (not trait objects) — `TaskStore`, `ManifestStore`
- Task ID resolution: exact ULID → exact short_id (SD42) → ULID prefix match. Duplicate short_ids produce an explicit error
- **Body editing**: `update` supports `--body-replace`, `--body-sub` (sed-style `s/pat/repl/[g]`), and `--dry-run`. Operation order: replace → sub → append → note. `body_edit.rs` has the sed parser

## TUI Modes

- **Normal** — task list navigation, sort/filter, actions (complete, delete, reload)
- **Searching** — `/` enters search input, `Enter` applies, `Esc` cancels
- **Creating** — `n` opens a multi-field form (title, priority, tags, stack, body); `Tab`/`Shift+Tab` navigate fields, `Enter` submits
- **Settings** — `,` opens settings (default sort, default filter, auto-capture git, quick list limit); `s` saves to manifest
- **FilterMenu** — status filter cycling via `f`

State lives in `tui::app::App`; rendering in `tui::ui`; key handling in `tui::handler`.

## Conventions

- Error handling: `thiserror` for domain errors, `anyhow` available for ad-hoc use
- `dir_context::capture_full()` returns `CaptureResult` (context + config file path); `capture()` is the shorthand returning just `ContextInfo`
- MCP server uses rmcp with `#[tool_handler]` and `#[prompt_handler]` proc macros on the `ServerHandler` impl

## Dogfooding

This project has a workspace-local task store at `.stackydo-workspace/` (gitignored). AI agents working on this codebase should use stackydo to track their own work. All commands use the `STACKYDO_DIR` env var to target the local workspace:

```bash
STD=".stackydo-workspace"  # shorthand for examples below

# Create a task for the work you're about to do
STACKYDO_DIR=$STD cargo run -- create --title "Implement feature X" --stack dev -- Description of the work

# Log progress with timestamped notes
STACKYDO_DIR=$STD cargo run -- update <ID> --note "Added tests, found edge case in parser"

# Mark done when finished
STACKYDO_DIR=$STD cargo run -- complete <ID>

# Check what's in the workspace
STACKYDO_DIR=$STD cargo run -- list --stack dev
```

**When to use it**: Non-trivial, multi-step work (features, refactors, bug investigations). Don't bother for one-line fixes.

**When something goes wrong**: If a stackydo command fails, returns wrong data, panics, or behaves unexpectedly while you're using it:

1. **Interactive session** — Tell the user immediately. They need to know.
2. **Can use `gh`** — File an issue: `gh issue create --title "Bug: ..." --body "..."`
3. **Fallback** — Write a bug report to `bugs/<YYYY-MM-DD>-<short-slug>.md` in the repo root (create `bugs/` if needed). Include: what you ran, what happened, what you expected, and any error output.
