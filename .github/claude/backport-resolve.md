For each PR, do the following:

1. Create a local branch tracking `origin/<branch>`.
2. Resolve every conflict marker in the files.
3. Handle each file listed for the PR under "Files the backport removed". The original change to it could not be merged,
   so the backport deleted the file. The backport commit's message names the cherry-picked commit;
   `git show <that commit> -- <path>` shows the original change. Do not add the file back.
   - `Backport-Missing: <path>`: the original PR changed this file, but the target branch does not have it. Find where
     the target branch has the code that the original change touched, and apply the change there if the target needs it.
   - `Backport-Deleted: <path>`: the original PR deleted this file, but the target branch had changed it
     (`git log -p origin/<target> -- <path>` shows how, with `<target>` from the branch name
     `backport-<pr>-to-<target>`). Find where the original PR moved the file's code, if anywhere, and carry over the
     target branch's changes to it if the target needs them.
4. Run `just fmt`.
5. Stage all changes with `git add -A`.
6. Commit. If the target needs none of the changes from step 3 and there were no conflict markers, commit with
   `--allow-empty` and say why in the message.
7. Run `git status` to confirm the branch is clean, and `git log` to confirm your new commit is on top.
8. Write a concise markdown summary of the conflicts and how you resolved them to `claude-summaries/<pr-number>.md`
   (create the directory first).

Rules:

- Do not push and do not use the `gh` CLI; the workflow verifies compilation and pushes.
- Do not commit the `claude-summaries/` files.
- Only create new commits authored by the current git identity (the bot). Do not use `--amend`, `--author=`,
  `--reuse-message=`, `git rebase`, or `git cherry-pick --continue`.
