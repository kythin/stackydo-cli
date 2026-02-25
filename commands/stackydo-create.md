---
name: stackydo-create
description: Create a new stackydo task from the current conversation context
model: sonnet
---

# Create Stackydo Task

You are helping the user create a new task in stackydo. Gather the necessary information and create the task using the CLI.

## Steps

1. If the user provided a title or description inline (e.g., `/stackydo-create Fix the login bug`), use that directly.
2. Otherwise, infer a task from the current conversation context — what was the user just working on or discussing?
3. Determine appropriate values:
   - **Title**: concise, imperative (e.g., "Fix auth token expiry handling")
   - **Priority**: low, medium, high, or critical (default: medium)
   - **Stack**: infer from context (e.g., "dev", "bugs", "docs") or ask
   - **Tags**: relevant keywords
   - **Body**: additional context, steps, or notes

4. Run the create command:
   ```bash
   stackydo create --title "<title>" --priority <priority> --stack <stack> --tags <tag1>,<tag2> -- <body>
   ```

5. Report the created task ID back to the user.

## Notes

- If `$STACKYDO_DIR` is set, tasks go there. Otherwise `~/.stackydo/`.
- The tool auto-captures git context (branch, repo, recent commits) at creation time.
- Use `$STACKYDO_LAST_ID` to chain related tasks in the same session.
