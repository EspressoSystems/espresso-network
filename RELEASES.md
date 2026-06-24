# Releases

This document describes the release process for the Espresso Network. It covers
how to cut a release, tag builds, promote them through staging environments,
manage backports, and handle release-associated experimental branches.

## Versioning

Every release tag has the form `MAJOR.MINOR.PHASE.PATCH` — for example
`0.4.0.7`. `MAJOR.MINOR` tracks the protocol version. `PHASE` distinguishes
between distinct release branches for the same protocol version (e.g. decaf vs
mainnet, pre- vs post-upgrade). `PATCH` is a monotonically increasing integer
within a single release branch, assigned each time `/tag` is invoked.

Tags are immutable. If a tag turns out to be bad, the next `/tag` produces the
next patch number; the old tag simply never gets promoted.

## Release branches

The canonical release branch is named **`release-MAJOR.MINOR.PHASE`** — e.g.
`release-0.4.0`. Branch protection rules require all changes to land via
reviewed PR; nothing is pushed directly.

When a `release-MAJOR.MINOR.PHASE` branch is created, automation does the
following:

1. Creates a label `backport-to-release-MAJOR.MINOR.PHASE`.
2. Opens a **release tracker issue** titled `Release MAJOR.MINOR.PHASE` (see
   below).

When such a branch is deleted, automation closes the tracker issue with a
final comment summarizing the tags that were produced and marks it
`release-scrapped`. Existing tags remain in the registry.

### Experimental branches off a release branch

Validation and experiment work happens on branches named
**`release-MAJOR.MINOR.PHASE/<topic>`** — e.g. `release-0.4.0/ff-base-testing`.
These match the existing `release-*` CI trigger, so they get full docker
builds for devnet testing, but the tag/tracker automation explicitly ignores
them: no `/tag` command works against them, no tracker issue is created.

Workflow: branch off the canonical release branch → push changes → CI produces
a docker image you can deploy on devnet → validate → open a PR back into the
canonical release branch. The experimental branch is deleted once merged or
when it turns out the experiment didn't pan out.

## The release tracker issue

The tracker issue is the durable, branch-scoped surface for everything
release-related. It is the place where you run release commands, see status,
and discuss the release with the team. Its body is maintained automatically;
the bot edits sections between `<!-- BEGIN ... -->` / `<!-- END ... -->`
markers, so anything you write outside those markers is preserved.

Sections in the body:

- **Tag log** — chronological list of tags cut from this branch (sha, time, who triggered).
- **Promotion state** — which tag is currently at `decaf.rc`, `decaf`, `mainnet.rc`, `mainnet`.
- **Backport candidates** — commits on `main` not yet in this branch (checklist).
- **Forward-port candidates** — commits on this branch not yet in `main`.
- **Skip list** — shas explicitly excluded from the candidate lists, with reason and who skipped them.
- **Experimental branches** — currently-open branches matching `release-MAJOR.MINOR.PHASE/*` with their last-commit time and link, so you can see what's being validated at a glance.

## Cutting a release: end-to-end

1. **Cut the branch.** From `main` at the chosen commit, push a new branch
   `release-MAJOR.MINOR.PHASE`. Automation creates the backport label and
   tracker issue. The tracker issue is where you'll do everything that
   follows.

2. **Run an initial tag.** Comment `/tag` on the tracker issue. This creates
   tag `MAJOR.MINOR.PHASE.0`, which triggers `build.yml` to publish docker
   images for every service.

3. **Iterate.** Land backport PRs (see the next section). After each batch of
   PRs that you judge ready to deploy, comment `/tag` again to cut the next
   patch.

4. **Validate on devnet.** Either deploy the new tag directly, or create an
   experimental branch off the release branch with additional ad-hoc changes
   and deploy that.

5. **Promote.** Once a tag is validated, comment `/promote <stage>` on the
   tracker — `<stage>` is one of `decaf.rc`, `decaf`, `mainnet.rc`, `mainnet`.
   This promotes the **most recent tag** from the branch into the floating
   docker tag for that stage. To promote an older tag, run the
   `promote-docker-tag.yml` workflow directly via `gh` or the Actions UI.

6. **Repeat the promotion sequence.** The intended progression is
   `decaf.rc → decaf → mainnet.rc → mainnet`. Each stage may be a different
   tag if you cut additional patches between promotions.

7. **Move on.** Once a release is shipped to mainnet and the chain has
   advanced past the upgrade point, the next phase's branch
   (`release-MAJOR.MINOR.<PHASE+1>`) is cut from the current one.

## Promotion approval

Promotion to each stage is gated by **GitHub Environment required reviewers**,
configured in repo Settings → Environments:

- `decaf.rc` — 2 reviewers
- `decaf` — 2 reviewers
- `mainnet.rc` — 3 reviewers
- `mainnet` — 3 reviewers

All four environments have "Prevent self-review" enabled, so whoever runs
`/promote` cannot approve their own promotion. When you run `/promote`, the
workflow run pauses in the Actions UI; configured reviewers receive a "Review
pending deployments" notification and approve from there. The bot posts the
result back to the tracker issue when the workflow completes.

Reviewer rosters are managed in the GitHub UI — changes do not require a code
change.

## Backports

Backports are label-driven. To request that a PR be backported to an active
release, add the label `backport-to-release-MAJOR.MINOR.PHASE` to it. When the
PR merges, `backport.yml` automatically opens a draft backport PR against the
release branch, cherry-picking the merge with `-x`. If the cherry-pick has
conflicts, the workflow attempts to resolve them automatically (mergiraf
first, then Claude); resulting PRs are labeled `claude-resolved` so they get
extra scrutiny.

The tracker issue's **Backport candidates** section lists commits on `main`
that haven't been backported yet. To explicitly mark a commit as "not for
backport", comment `/skip <sha> <reason>` on the tracker; the commit moves
into the **Skip list** and disappears from the candidate list. Use
`/unskip <sha>` to reverse.

`/skip` also applies to forward-port candidates by sha — the same command
covers both directions.

## Quick reference: tracker commands

| Command | Effect |
|---|---|
| `/tag` | Cut the next patch tag from the tip of this release branch. |
| `/promote <stage>` | Promote the most recent tag from this branch to `<stage>` (one of `decaf.rc`, `decaf`, `mainnet.rc`, `mainnet`). Gated by environment reviewers. |
| `/skip <sha> [reason]` | Remove `<sha>` from the backport/forward-port candidate lists. |
| `/unskip <sha>` | Reverse a previous `/skip`. |

All commands require repo write access (`OWNER`, `MEMBER`, or `COLLABORATOR`).

## Manual fallback

If the tracker issue is unavailable, the same operations are available via the
GitHub CLI:

```bash
just tag release-0.4.0                                # same as /tag
gh workflow run promote-docker-tag.yml \              # same as /promote decaf.rc
  -f stage=decaf.rc -f tag=0.4.0.5
```
