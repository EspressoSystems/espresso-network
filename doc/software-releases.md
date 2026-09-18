# Software Releases

Release tags have the form `MAJOR.MINOR.PHASE.PATCH`, for example `0.6.0.7`.

| Part          | Meaning                                                                                                         |
| ------------- | --------------------------------------------------------------------------------------------------------------- |
| `MAJOR.MINOR` | Protocol version the release branch implements (`0.6` for `V0_6`).                                              |
| `PHASE`       | Distinguishes release branches for the same protocol version: pre vs post activation, or a new cut from `main`. |
| `PATCH`       | Increments with every tag on the branch. `.0` marks the cut point.                                              |

Tags are immutable. A bad tag is superseded by the next one and never deployed.

Docker images are tagged with the git tag: git tag `0.6.0.7` produces
`ghcr.io/espressosystems/espresso-network/espresso-node:0.6.0.7`.

## Branches

- Release branches are named `release-MAJOR.MINOR.PHASE`, e.g. `release-0.6.0`, cut from `main` with the
  [Release Branch](../.github/workflows/release-branch.yml) workflow. All changes land via reviewed PR.
- Experimental branches `release-MAJOR.MINOR.PHASE--<topic>` (double dash) branch off a release branch for devnet
  validation. CI builds docker images for them. Release automation ignores them.
- Backports: add the label `backport release-MAJOR.MINOR.PHASE` to a PR on `main`. On merge, `backport.yml` opens a
  backport PR against the release branch and tries to resolve conflicts (PRs it touched carry the label
  `claude-resolved`). Manual backports use `git cherry-pick -x`.

## Tracker issue

Every release branch has one issue titled `Release MAJOR.MINOR.PHASE` with label `release-tracker`. The bot regenerates
its body on every push to `main` or `release-*`, after every `/tag` or cut, and on tracker commands. Sections:

- Tag log: tags on the branch with date and commit.
- Commits on `main` not yet on the branch: checklist since the `.0` cut point. Boxes tick automatically when
  `git cherry` finds the patch on the release branch. Backport PR status is appended when one exists.
- Commits on the branch: checklist of what landed on the release branch since the cut.
- Experimental branches: open `release-X.Y.Z--*` branches with their tip.
- Human notes: free text below `<!-- HUMAN NOTES BELOW -->`, preserved verbatim.

Commands are comments on the tracker issue by an org member or repo collaborator (GitHub `author_association` `OWNER`,
`MEMBER` or `COLLABORATOR`); comments by others are ignored. `<sha>` is a commit sha prefix of at least 7 characters.

| Command         | Effect                                                                  |
| --------------- | ----------------------------------------------------------------------- |
| `/tag`          | Tag the branch tip with the next patch, create a GitHub Release, build. |
| `/tag X.Y.Z.N`  | Same with an explicit tag. Must match the branch version and be new.    |
| `/done <sha>`   | Tick a commit that was ported outside the backport workflow.            |
| `/skip <sha>`   | Strike through a commit that is deliberately not ported.                |
| `/unmark <sha>` | Undo `/done` or `/skip`.                                                |

Marks are replayed from the issue's comment history, so the body can always be regenerated.

## Process

1. Cut the branch: run the Release Branch workflow with `version` (e.g. `0.6.0`) and `source_ref` (default `main`), or
   `just release-cut 0.6.0`. This pushes `release-0.6.0`, tags `0.6.0.0`, creates the backport label and the tracker
   issue, and builds images.
2. Land backport PRs and fixes on the release branch. Watch the tracker checklist.
3. Comment `/tag` on the tracker after each batch worth deploying. The bot replies with the tag, the GitHub Release and
   a link to the `build.yml` run.
4. Validate the tag on devnet, or push an experimental branch for ad hoc changes.
5. Announce the release with the GitHub Release link.
6. Delete the release branch when it is no longer needed, shipped or abandoned. The tracker is closed with a final tag
   list and labelled `release-closed`.

## Workflows

| Workflow                     | Trigger                                             | Runs                              |
| ---------------------------- | --------------------------------------------------- | --------------------------------- |
| `release-branch.yml`         | `workflow_dispatch`, branch `delete`                | `scripts/release cut`, `teardown` |
| `tag-release.yml`            | `/tag` comment, `workflow_dispatch`                 | `scripts/release tag`             |
| `update-release-tracker.yml` | push to `main`/`release-*`, mark comments, dispatch | `scripts/release refresh`         |
| `build.yml`                  | dispatched by `cut` and `tag` for the new tag       | docker images                     |

Tags pushed by workflows do not fire `push` events, so `cut` and `tag` dispatch `build.yml` and the tracker refresh
explicitly.

## Local use

All logic is in `scripts/release` (stdlib Python, tests in `scripts/test_release.py`, run with `just py::test`). It
needs `gh auth` and an `origin` remote.

```sh
just release-body 0.6.0          # render the tracker body to stdout, no writes
just release-tag release-0.6.0   # same as commenting /tag
just release-tag release-0.6.0 0.6.0.5
scripts/release tag --branch release-0.6.0 --dry-run   # show the tag, sha and tracker comment, no writes
scripts/release --help
```

## Protection

- `/tag` and the mark commands require org membership or collaborator status; `workflow_dispatch` requires write access.
  Tag protection rules cannot exempt the workflow token, so tags are not protected yet.
- Floating docker tags per network (`decaf`, `mainnet`) and automated promotion are not part of this process. Operators
  pin release tags.
