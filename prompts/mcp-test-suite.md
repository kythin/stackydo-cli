# Stackydo MCP Test Suite Prompt

> Paste this prompt to run a comprehensive test of the stackydo-mcp server via the MCP tools.
> Assumes the MCP server is connected as `stackydo-cargo` and the CWD has a `stackydo.json`.

---

You are a test analyst with access to the stackydo-cargo MCP tools. Run a comprehensive test suite against the MCP server and write your findings to `./test-results-[YYYY-MM-DD].md`. Clean up all test data when done.

## Test Areas

### 1. Storage Location (Critical)
- Create a task via MCP and verify the `.md` file lands in the project workspace (per `stackydo.json` `dir` field), NOT in `~/.stackydo/`
- Use `list_workspaces` to confirm both workspaces are visible
- Check with `ls` that no files leaked to the global workspace
- Verify hard delete actually removes the file from disk

### 2. Unicode / Emoji / Special Characters
- Create tasks with: emoji in title/body, CJK characters, accented unicode (`cafe resume naive`), HTML entities (`<script>` tags), YAML-hostile body content (`---`, `key: value`), special shell chars (`& < > " ' / \ | ; $ ! @ # % ^ * ( ) { } [ ]`)
- Verify round-trip: create -> get -> confirm content matches

### 3. Filtering
- Test all filters: `status`, `priority`, `tag`, `stack`, `overdue`, `due_before`, `due_after`
- Test invalid values for each filter (should return errors)
- Test filter combinations

### 4. Sorting
- Test all sort values: `created`, `due`, `modified`, `priority`
- Test invalid sort value — check if it errors or silently falls back
- Verify sort order is correct (check first/last items)

### 5. Pagination
- Create 10+ tasks, then test `limit` + `offset` for page 1, page 2, last page
- Test `offset` beyond total (should return `[]`)
- Test `limit=0` (no limit)
- Test default limit (omit param, should be 50)

### 6. Search
- Test text match in title and body
- Test emoji/unicode search
- Test no-match query (should return `[]`)
- Test empty string query
- Test search combined with filters, sort, pagination, group_by

### 7. Group By
- Test `group_by=stack` — verify grouping, `(no stack)` key for unstacked tasks
- Test invalid `group_by` value
- Test `group_by` with `full=true`

### 8. CRUD Lifecycle
- **Create**: all fields, minimal fields, invalid priority, invalid due date, whitespace-only title, empty title
- **Get**: full ID, prefix match, nonexistent ID
- **Update**: title, status, priority, tags (set/clear), stack (set/clear), due (set/clear), note append, no-change update
- **Complete**: verify status change
- **Delete**: verify file removal, verify task no longer appears in list

### 9. Edge Cases
- Very long title (400+ chars)
- Very long body with markdown (tables, code blocks, lists, headings)
- Newlines in note text
- `full=true` vs `full=false` (default) on list output
- Stats endpoint accuracy after mixed operations
- Stacks endpoint with multiple stacks

## Output Format

Write results to `./test-results-[YYYY-MM-DD].md` with:
- Test environment info
- Table per test area: Test | Result (PASS/FAIL/BUG/NOTE) | Notes
- Bugs section with severity, location (file:line), description, and fix suggestion
- Design notes section for non-bugs worth discussing
- Summary table

## Cleanup

Delete ALL test tasks when done. Verify with `list_tasks` that the workspace is back to its pre-test state.
