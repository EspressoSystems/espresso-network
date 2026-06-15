This PR is a backport. The original PR number is in `./tmp/is-backport`.

Perform a BACKPORT-CORRECTNESS review. Do not perform a general code review.

- `./tmp/original.diff` is the original PR's diff; `./tmp/backport.diff` is this PR's diff. Compare them and confirm the
  backport faithfully reproduces the original change, adapted correctly to the target branch (API/type differences
  between branches, conflict-resolution edits, dropped or extra hunks).
- `./tmp/original-pr-comments.json` holds the original PR's review comments. Do NOT re-flag anything already raised
  there. Only flag problems introduced by the backport itself (bad merge, lost hunk, wrong adaptation).
