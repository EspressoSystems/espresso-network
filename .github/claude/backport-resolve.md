For each PR: create a local branch tracking origin/<branch>, resolve the markers, run `just fmt`, commit. Do not push
and do not use the `gh` CLI; the workflow will verify compilation and push.

Commit attribution: only create new commits authored by the current git identity (the bot). Do not use --amend,
--author=, --reuse-message=, git rebase, or `git cherry-pick --continue`; those preserve the original PR author's
identity, which must never happen here.

For each PR, write a concise markdown summary of the conflicts and how you resolved them to
`claude-summaries/<pr-number>.md` (create the directory first; do not commit these files).
