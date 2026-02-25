---
name: stackydo-plan
description: Break a goal into actionable stackydo tasks with dependencies
model: sonnet
---

# Plan Work with Stackydo

Break a user's goal into a structured set of stackydo tasks with priorities, dependencies, and a suggested execution order.

## Steps

1. Understand the goal. If the user passed it inline (e.g., `/stackydo-plan Implement user auth`), use that. Otherwise, ask.

2. Check existing tasks for overlap:
   ```bash
   stackydo search "<relevant keywords>" --json
   ```

3. Break the goal into 3-8 concrete, actionable tasks. For each task determine:
   - **Title**: imperative, specific
   - **Priority**: based on dependency order and importance
   - **Stack**: consistent grouping for this body of work
   - **Dependencies**: which tasks block others
   - **Body**: acceptance criteria or key details

4. Present the plan to the user for review before creating anything.

5. Once approved, create the tasks:
   ```bash
   stackydo create --title "<title>" --priority <pri> --stack <stack> -- <body>
   ```

6. Wire up dependencies using `--blocked_by` and `--blocks` on subsequent `stackydo update` calls.

7. Show the final task list with IDs so the user can reference them.

## Notes

- Don't over-plan. 3-8 tasks is the sweet spot. More than that and it becomes noise.
- Use a single stack for the plan so tasks can be filtered together.
- Set the first task(s) to `open` and dependent ones can stay `open` with `blocked_by` set.
