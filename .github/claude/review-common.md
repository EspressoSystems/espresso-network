Before reviewing, read existing comments from `./tmp/pr-review-comments.json`.

- Do not duplicate feedback already given by any reviewer.
- For your own unresolved review threads: respond to any changes or replies, then resolve if the issue is fixed,
  explained, or answered satisfactorily.
  `gh api graphql -f query='mutation { resolveReviewThread(input: {threadId: "<thread_id>"}) { thread { isResolved } } }'`
- Only resolve your own threads, never others'.

Do NOT comment on:

- `todo!()` or `// TODO` -- these are intentional, just factor them into your understanding
- Unnecessary clones or clone optimization suggestions
- Unused code, fields, or parameters -- clippy catches these, and `_` prefixed names are intentionally unused

Provide detailed feedback using inline comments for specific issues. Use top-level comments for general observations or
praise.
