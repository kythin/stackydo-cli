# CLAUDE.md

## Project Overview

`stackstodo` is a context-aware CLI task manager written in Rust. Tasks are stored as individual markdown files with YAML frontmatter in `~/.stackstodo/` (overridable via `$STACKSTODO_DIR`). It supports both a ratatui-based TUI (default) and headless CLI commands. Binaries: `stackstodo` (primary) and `todo` (alias).

## Build & Run

```bash
cargo build
cargo run              # launches TUI
cargo run -- create --title "My task" --stack "work" -- body content here
cargo run -- list
cargo run -- list --stack work
cargo run -- search "query"
cargo run -- context   # debug: show what context would be captured
cargo test
cargo clippy
```

## Testing

```bash
cargo test                                # unit tests (in-module #[cfg(test)])
cargo build && bash tests/smoke_test.sh   # CLI smoke tests (49 assertions)
```

The smoke test sets `STACKSTODO_DIR=tests/.test-data/` so it never touches `~/.stackstodo/`. The test data directory is gitignored and cleaned up automatically.

Unit tests use `tempfile` crate for filesystem isolation.

## Architecture

- **`src/model/`** — Core types: `Task`, `TaskFrontmatter`, `Manifest`, `ContextInfo`, status/priority/dependency enums
- **`src/storage/`** — Filesystem persistence: markdown+YAML task files, JSON manifest, path resolution
- **`src/context/`** — Auto-capture: git info (git2), `.stackstodo-context` file discovery, directory context
- **`src/cli/`** — clap derive argument definitions
- **`src/commands/`** — Headless command implementations (create, list, show, complete, delete, search, context)
- **`src/tui/`** — ratatui TUI: app state, 40/60 split layout, keybinding handler, widgets
- **`src/error.rs`** — `TodoError` enum using thiserror

## Key Design Decisions

- Tasks are flat files: `<STACKSTODO_DIR>/<ULID>.md` — no database
- `$STACKSTODO_DIR` env var overrides the default `~/.stackstodo/` storage root; all path resolution goes through `TodoPaths::root()`
- ULIDs for IDs (time-sortable, unique)
- All dates stored as UTC, displayed in local timezone
- YAML frontmatter is managed by the tool; body content is freeform
- `.stackstodo-context` files discovered by walking up from CWD, fallback `~/.stackstodo-context`
- `$STACKSTODO_LAST_ID` env var chains tasks created in the same shell session
- Task graph: subtasks (parent_id/subtask_ids) + dependencies (blocked_by, blocks, related_to)
- **Stacks**: each task has an optional `stack: Option<String>` field (one stack per task); manifest tracks known stack names

## TUI Modes

- **Normal** — task list navigation, sort/filter, actions (complete, delete, reload)
- **Searching** — `/` enters search input, `Enter` applies, `Esc` cancels
- **Creating** — `n` opens a multi-field form (title, priority, tags, stack, body); `Tab`/`Shift+Tab` navigate fields, `Enter` submits
- **Settings** — `,` opens settings (default sort, default filter, auto-capture git, quick list limit); `s` saves to manifest
- **FilterMenu** — status filter cycling via `f`

State lives in `tui::app::App`; rendering in `tui::ui`; key handling in `tui::handler`.

## Conventions

- Error handling: `thiserror` for domain errors, `anyhow` available for ad-hoc use
- Storage layer uses concrete types (not trait objects) for simplicity — `TaskStore`, `ManifestStore`
- Task ID resolution supports prefix matching (e.g. `stackstodo show 01HQ` matches the full ULID)
- `dir_context::capture_full()` returns `CaptureResult` (context + config file path); `capture()` is the shorthand returning just `ContextInfo`
