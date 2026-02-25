---
name: stackydo-standup
description: Generate a daily standup summary from stackydo tasks
model: sonnet
---

# Daily Standup

Generate a quick standup summary from the user's stackydo tasks.

## Steps

1. Get stats overview:
   ```bash
   stackydo stats --json
   ```

2. List recently completed tasks (done status):
   ```bash
   stackydo list --status done --sort modified --limit 10 --json
   ```

3. List in-progress tasks:
   ```bash
   stackydo list --status in_progress --json
   ```

4. List blocked tasks:
   ```bash
   stackydo list --status blocked --json
   ```

5. Check overdue items:
   ```bash
   stackydo list --overdue --json
   ```

6. Present a standup report in this format:

   **Done** (recently completed)
   - Task 1
   - Task 2

   **In Progress**
   - Task 3 (stack: dev)

   **Blocked**
   - Task 4 — reason if available

   **Heads Up**
   - N overdue tasks
   - Any other flags

## Notes

- Keep it brief — this is a standup, not a full review.
- Filter "done" tasks to only those completed recently (check modified date).
- If a stack filter is relevant to the user's current context, apply it.
