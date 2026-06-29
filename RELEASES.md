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

1. Creates a label `backport release-MAJOR.MINOR.PHASE`.
2. Tags the branch tip as `MAJOR.MINOR.PHASE.0` — the initial release tag,
   which also serves as the cut-point reference for the tracker.
3. Opens a **release tracker issue** titled `Release MAJOR.MINOR.PHASE` (see
   below).

When such a branch is deleted, automation closes the tracker issue with a
final comment summarizing the tags that were produced and marks it
`release-scrapped`. Existing tags remain in the registry.

### Experimental branches off a release branch

Validation and experiment work happens on branches named
**`release-MAJOR.MINOR.PHASE--<topic>`** — e.g. `release-0.4.0--ff-base-testing`. (Double-dash separator: git forbids ref overlap, so a `release-0.4.0/...` branch can't exist while `release-0.4.0` itself does.)
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
and discuss the release with the team. The body is a pure projection: the
bot regenerates it from authoritative sources (git refs, GHCR manifest
digests, the GitHub branches/PRs API, and the tracker's own comment history)
on every relevant event. Free-form notes go below the `<!-- HUMAN NOTES BELOW -->`
sentinel at the bottom of the body — everything after that marker is
preserved verbatim across refreshes.

Sections in the body, in order:

- **Tag log** — chronological list of tags cut from this branch (sha, time, who triggered).
- **Promotion state** — which tag is currently at `decaf.canary`, `decaf`, `mainnet.canary`, `mainnet`.
- **Commits on `main`** — checklist of commits landed on `main` since the branch was cut, with the corresponding backport-PR status when one exists. Boxes auto-tick when the patch already exists on the release branch (clean cherry-pick or backport merge). Comment `/done <sha>` to tick manually-ported commits the auto-detection misses, or `/skip <sha>` to strike through commits you're deliberately ignoring.
- **Commits on `release-X.Y.Z`** — checklist of commits landed on this branch since it was cut. Same auto-tick / `/done` / `/skip` model.
- **Experimental branches** — currently-open branches matching `release-MAJOR.MINOR.PHASE--*` with their last commit, so you can see what's being validated at a glance.

## Cutting a release: end-to-end

1. **Cut the branch.** From `main` at the chosen commit, push a new branch
   `release-MAJOR.MINOR.PHASE`. Automation creates the backport label, tags
   the cut point as `MAJOR.MINOR.PHASE.0` (which fires `build.yml` and
   publishes the initial docker images), and opens the tracker issue. The
   tracker is where you'll do everything that follows.

2. **Iterate.** Land backport PRs (see the next section). After each batch
   of PRs that you judge ready to deploy, comment `/tag` on the tracker to
   cut the next patch (`.1`, `.2`, …) or `/tag X.Y.Z.N` to name an explicit
   one.

3. **Validate on devnet.** Either deploy the new tag directly, or create an
   experimental branch off the release branch with additional ad-hoc changes
   and deploy that.

4. **Promote.** Once a tag is validated, comment `/promote <stage>` on the
   tracker (`<stage>` is one of `decaf.canary`, `decaf`, `mainnet.canary`,
   `mainnet`). That promotes the **most recent tag** from the branch into
   the floating docker tag for that stage. To pin an older tag at a stage
   that's lagging behind your latest tag (e.g. keep `mainnet` on `.3` while
   you canary-test `.5` on `decaf.canary`), use `/promote <tag> <stage>`.

5. **Across stages.** The conventional progression is
   `decaf.canary → decaf → mainnet.canary → mainnet`, but it isn't enforced
   — stages are independent. The reviewer gate on each environment is the
   real control; promote ordering is a convention reviewers should check.

6. **Move on.** Once a release is shipped to mainnet and the chain has
   advanced past the upgrade point, the next phase's branch
   (`release-MAJOR.MINOR.<PHASE+1>`) is cut from the current one.

## Promotion approval

Promotion to each stage is gated by **GitHub Environment required reviewers**,
configured in repo Settings → Environments:

- `decaf.canary` — 2 reviewers
- `decaf` — 2 reviewers
- `mainnet.canary` — 3 reviewers
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
release, add the label `backport release-MAJOR.MINOR.PHASE` to it. When the
PR merges, `backport.yml` automatically opens a draft backport PR against the
release branch, cherry-picking the merge with `-x`. If the cherry-pick has
conflicts, the workflow attempts to resolve them automatically (mergiraf
first, then Claude); resulting PRs are labeled `claude-resolved` so they get
extra scrutiny.

The tracker issue's **Commits on `main`** section accumulates commits on
`main` as they land. Boxes auto-tick whenever the patch is already present on
the release branch (clean cherry-pick or backport-PR merge — detected via
`git cherry`). For commits ported outside that workflow (manual cherry-pick
that doesn't preserve the patch-id, manual reimplementation, etc.) comment
`/done <sha>` to tick the box. For commits you've decided *not* to backport,
comment `/skip <sha>` to strike the row through — the visual distinction
between "this is done" and "this is deliberately not done" tells a reviewer
at a glance what's left to consider. `/unmark <sha>` clears either mark.
Comment history is the authoritative log, so corruption or force-pushes to
the tracker body don't lose state.

If a release branch is force-pushed past the `.0` tag, the checklist
sections render "_Anchor `<sha>` is no longer reachable from `<head>` (branch
rewritten?)_". Recover by moving the `MAJOR.MINOR.PHASE.0` tag to the
appropriate new commit and pushing it.

## Quick reference: tracker commands

| Command | Effect |
|---|---|
| `/tag` | Cut the next patch tag from the tip of this release branch. |
| `/tag <tag>` | Cut the given tag from the tip. Passed through verbatim — `git tag` rejects malformed names or duplicates. |
| `/promote <stage>` | Promote the most recent tag from this branch to `<stage>` (one of `decaf.canary`, `decaf`, `mainnet.canary`, `mainnet`). Gated by environment reviewers. |
| `/promote <tag> <stage>` | Promote the specified `<tag>` to `<stage>`. Use this to keep an older tag pinned at a downstream stage after cutting newer ones. |
| `/done <sha>` | Mark a commit as ported (ticked box). Use when the auto-detector misses a manual port. |
| `/skip <sha>` | Mark a commit as deliberately not ported (struck through, no tick). |
| `/unmark <sha>` | Undo a prior `/done` or `/skip` for the same commit. |

All commands require repo write access (`OWNER`, `MEMBER`, or `COLLABORATOR`).

## Manual fallback

If the tracker issue is unavailable, the same operations are available via the
GitHub CLI:

```bash
# /tag — auto-pick next patch
just tag release-0.4.0

# /tag X.Y.Z.N — explicit tag name
gh workflow run tag-release.yml --ref release-0.4.0 -f tag=0.4.0.5

# /promote <tag> <stage>
gh workflow run promote-docker-tag.yml \
  -f floating-tag=decaf.canary -f release-tag=0.4.0.5
```

`/done`, `/skip`, and `/unmark` are tracker-only — there's no CLI equivalent
because their authority is the tracker's comment history.
