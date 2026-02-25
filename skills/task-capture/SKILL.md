---
name: task-capture
description: Automatically extract actionable items from conversation and create stackydo tasks. Use when the user says "capture tasks", "extract todos", or "log these as tasks".
allowed-tools: Bash, Read, Grep
---

# Task Capture

## Purpose
Extract actionable items from text, conversation context, or code comments and create stackydo tasks for each one.

## Trigger Keywords
- "capture tasks"
- "extract todos"
- "log these as tasks"
- "create tasks from this"
- "turn these into tasks"

## Process

1. **Identify actionable items** from the provided text or recent conversation:
   - Look for imperative statements ("Fix X", "Add Y", "Update Z")
   - Look for TODO/FIXME/HACK comments in code
   - Look for bullet lists of work items
   - Look for questions that imply missing functionality

2. **Deduplicate** against existing tasks:
   ```bash
   stackydo search "<keyword>" --json
   ```

3. **Categorize** each item:
   - Assign priority based on urgency signals (e.g., "critical", "blocking", "nice to have")
   - Assign stack based on domain (e.g., code references -> "dev", docs -> "docs")
   - Extract tags from context

4. **Create tasks** for each unique item:
   ```bash
   stackydo create --title "<title>" --priority <pri> --stack <stack> --tags <tags> -- <body>
   ```

5. **Report** what was created:
   - List each task with its ID, title, and stack
   - Note any items skipped as duplicates
   - Suggest next steps

## Guidelines

- Don't create tasks for things already done in the conversation
- Prefer fewer, well-defined tasks over many vague ones
- If unsure about priority or stack, default to medium/inbox
- Include enough body context that the task is actionable on its own
