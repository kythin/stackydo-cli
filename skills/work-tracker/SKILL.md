---
name: work-tracker
description: Track ongoing coding work in stackydo — create tasks before starting, log progress notes, and complete when done. Use when beginning multi-step development work.
allowed-tools: Bash, Read, Grep, Glob
---

# Work Tracker

## Purpose
Automatically track development work in stackydo as it happens. Creates a task when work begins, appends progress notes, and completes when finished.

## Trigger Keywords
- "track this work"
- "start tracking"
- "log my work"
- "begin task"

## Process

### Starting Work

1. **Create a task** for the work about to begin:
   ```bash
   stackydo create --title "<what>" --stack dev --priority medium -- <initial context>
   ```

2. **Store the task ID** for progress updates throughout the session.

### During Work

3. **Log milestones** as timestamped notes:
   ```bash
   stackydo update <ID> --note "<what was done or discovered>"
   ```

   Good triggers for notes:
   - Completed a significant sub-step
   - Found an unexpected issue or edge case
   - Changed approach from the original plan
   - Hit a blocker

### Finishing Work

4. **Complete the task** with a final note:
   ```bash
   stackydo update <ID> --note "Done: <summary of what was accomplished>"
   stackydo complete <ID>
   ```

5. **Create follow-up tasks** if anything was deferred:
   ```bash
   stackydo create --title "<follow-up>" --stack dev -- Deferred from <ID>: <reason>
   ```

## Environment

- Use `STACKYDO_DIR=.stackydo-workspace` for local workspace tracking
- Use default `~/.stackydo/` for personal task management
- Check `STACKYDO_DIR` first to match the user's preferred store

## Guidelines

- One task per logical unit of work (feature, bug fix, refactor)
- Don't create tasks for trivial changes
- Notes should be informative, not just "did stuff"
- If the work scope grows, create subtasks rather than one enormous task
