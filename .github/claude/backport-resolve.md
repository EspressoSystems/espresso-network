For each PR, do the following:

1. Create a local branch tracking `origin/<branch>`.
2. Resolve every conflict marker in the files.
3. Run `just fmt`.
4. Stage all changes with `git add -A`.
5. Commit.
6. Run `git status` to confirm the branch is clean, and `git log` to confirm your new commit is on top.
7. Write a concise markdown summary of the conflicts and how you resolved them to `claude-summaries/<pr-number>.md` (create the directory first).

Rules:

- Do not push and do not use the `gh` CLI; the workflow verifies compilation and pushes.
- Do not commit the `claude-summaries/` files.
- Only create new commits authored by the current git identity (the bot). Do not use `--amend`, `--author=`, `--reuse-message=`, `git rebase`, or `git cherry-pick --continue`.
