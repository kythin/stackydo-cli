# CLAUDE.md

## Project Overview

`todo` is a context-aware CLI task manager written in Rust. Tasks are stored as individual markdown files with YAML frontmatter in `~/.todos/`. It supports both a ratatui-based TUI (default) and headless CLI commands.

## Build & Run

```bash
cargo build
cargo run              # launches TUI
cargo run -- create --title "My task" -- body content here
cargo run -- list
cargo run -- search "query"
cargo test
cargo clippy
```

## Architecture

- **`src/model/`** — Core types: `Task`, `TaskFrontmatter`, `Manifest`, `ContextInfo`, status/priority/dependency enums
- **`src/storage/`** — Filesystem persistence: markdown+YAML task files, JSON manifest, path resolution
- **`src/context/`** — Auto-capture: git info (git2), `.todo-context` file discovery, directory context
- **`src/cli/`** — clap derive argument definitions
- **`src/commands/`** — Headless command implementations (create, list, show, complete, delete, search)
- **`src/tui/`** — ratatui TUI: app state, 40/60 split layout, keybinding handler, widgets
- **`src/error.rs`** — `TodoError` enum using thiserror

## Key Design Decisions

- Tasks are flat files: `~/.todos/<ULID>.md` — no database
- ULIDs for IDs (time-sortable, unique)
- All dates stored as UTC, displayed in local timezone
- YAML frontmatter is managed by the tool; body content is freeform
- `.todo-context` files discovered by walking up from CWD, fallback `~/.todo-context`
- `$TODO_LAST_ID` env var chains tasks created in the same shell session
- Task graph: subtasks (parent_id/subtask_ids) + dependencies (blocked_by, blocks, related_to)

## Conventions

- Error handling: `thiserror` for domain errors, `anyhow` available for ad-hoc use
- Storage layer uses concrete types (not trait objects) for simplicity — `TaskStore`, `ManifestStore`
- TUI state is in `tui::app::App`; rendering in `tui::ui`; key handling in `tui::handler`
- Task ID resolution supports prefix matching (e.g. `todo show 01HQ` matches the full ULID)

## Testing

Unit tests live alongside their modules (e.g. `storage/task_store.rs` has `#[cfg(test)]` block). Use `tempfile` crate for filesystem tests to avoid touching real `~/.todos/`.
