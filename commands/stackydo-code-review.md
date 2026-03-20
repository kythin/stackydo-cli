---
name: stackydo-code-review
description: "Parallel codebase audit — spawns Explore agents per layer, logs findings as stackydo tasks"
allowed-tools:
  - Agent
  - Read
  - Glob
  - Grep
  - Bash
  - mcp__stackydo-cargo__list_tasks
  - mcp__stackydo-cargo__create_task
  - mcp__stackydo-cargo__search_tasks
  - mcp__stackydo-cargo__update_task
  - mcp__stackydo-cargo__get_task
---

# Code Review Audit

Run a parallel codebase audit, logging findings as stackydo tasks in the `audit` stack.

## Scope

Parse `$ARGUMENTS` as the audit scope. Default is `all`. Valid values: `all`, `storage`, `model`, `commands`, `mcp`, `context`.

If scope is invalid, print the valid options and stop.

## Agent Assignments

Spawn up to 3 Explore agents in parallel. Each agent must read ALL files in its assigned area and report findings as a structured list: `[severity] file:line — description` where severity is one of `critical`, `high`, `medium`, `low`.

### auditor-core
- `src/model/` — all files
- `src/storage/` — all files
- `src/error.rs`
- `Cargo.toml`
- Focus: data integrity, serialization correctness, error handling gaps, unsafe unwraps, missing validations, dependency issues

### auditor-interface
- `src/commands/` — all files
- `src/cli/` — all files
- `src/main.rs`
- `tests/` — all files
- Focus: argument validation, output correctness, test coverage gaps, edge cases, UX issues

### auditor-integration
- `src/mcp/` — all files
- `src/mcp_bin.rs`
- `src/context/` — all files
- Focus: protocol compliance, context capture accuracy, error propagation, security (path traversal, injection)

### Scope Mapping
- `all` — spawn all 3 agents
- `storage` or `model` — spawn auditor-core only
- `commands` — spawn auditor-interface only
- `mcp` or `context` — spawn auditor-integration only

## Consolidation

After all agents report back:

1. Collect all findings into a single list
2. For each finding:
   a. Search existing stackydo tasks with `search_tasks` using the `audit` stack to check for duplicates (match on file + description similarity)
   b. If a duplicate exists and the new finding adds information: update the existing task with a note
   c. If new: create a new task with:
      - `stack`: `audit`
      - `priority`: `critical` or `high` severity -> `high`; `medium` -> `medium`; `low` -> `low`
      - `tags`: include the layer name (e.g. `core`, `interface`, `integration`) and affected module
      - `body`: full finding details including file path, line number, description, and suggested fix if obvious

## Summary

Present a final summary table:
- Total findings by severity
- New tasks created vs existing tasks updated
- Top critical/high items listed explicitly
