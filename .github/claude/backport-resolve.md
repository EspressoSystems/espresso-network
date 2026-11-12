For each PR, do the following:

1. Create a local branch tracking `origin/<branch>`.
2. Resolve every conflict marker in the files.
3. Handle each file listed for the PR under "Files without conflict markers". Git could not merge these files and left
   no markers in them. The backport commit's message names the cherry-picked commit (`<commit>` below), and
   `git show <commit> -- <path>` shows the original change. If the original PR renamed the file,
   `git show -M --stat <commit>` names the old path, and `git show -M <commit> -- <old path> <path>` shows the change.
   `<target>` is the target branch, from the branch name `backport-<pr>-to-<target>`.
   - `Backport-Missing: <path>`: the original PR changed this file, but the target branch does not have it, so the
     backport deleted it. Find where the target branch has the code that the original change touched, and apply the
     change there if the target needs it. Do not add the file back.
   - `Backport-Deleted: <path>`: the original PR deleted this file, but the target branch had changed it, so the
     backport deleted it. `git log -p $(git merge-base <commit>^ origin/<target>)..origin/<target> -- <path>` shows the
     target branch's own changes. Find where the original PR moved the file's code, if anywhere, and carry over those
     changes if the target needs them. Do not add the file back.
   - `Backport-Unresolved: <path>`: a binary file that both sides changed (it holds the target branch's version), or a
     file that both sides renamed (both names are present). Apply the original change if the target needs it, and keep
     one copy.
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
