# Software Releases

## Operators

- The latest release for a network has the network in its title, e.g. `0.6.3.16 [decaf]`.
- Mainnet: <https://github.com/EspressoSystems/espresso-network/releases/latest>
- Both:

  ```sh
  curl -fsS 'https://api.github.com/repos/EspressoSystems/espresso-network/releases?per_page=100' | jq -r '.[] | select(.name | test("\\[")) | .name'
  ```

- Image: `ghcr.io/espressosystems/espresso-network/espresso-node:<tag>`.
- Required actions: the release body, section for your network.
- Alerts: Watch > Custom > Releases. An alert means the release is ready to deploy on the network in its title.

## Tags

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

- Release branches are named `release-MAJOR.MINOR.PHASE`, e.g. `release-0.6.0`, cut from `main` with `just release-cut`.
  All changes land via reviewed PR.
- Experimental branches `release-MAJOR.MINOR.PHASE--<topic>` (double dash) branch off a release branch for devnet
  validation. CI builds docker images for them. Release automation ignores them.
- Backports: add the label `backport release-MAJOR.MINOR.PHASE` to a PR on `main`. On merge, `backport.yml` opens a
  backport PR against the release branch and tries to resolve conflicts (PRs it touched carry the label
  `claude-resolved`). After the merge, comment `/backport 123` on the tracker instead. Manual backports use
  `git cherry-pick -x`.

## Tracker issue

Every release branch has one issue titled `Release MAJOR.MINOR.PHASE` with label `release-tracker`. The bot regenerates
its body on every push to `main` or `release-*`, when a PR against a release branch opens or closes, after every `/tag`
or cut, and on tracker commands. Sections:

- Deployed line: the release each network runs.
- Tag log: tags on the branch with date, commit, GitHub release (`-` until published, then a link with the networks),
  new operator-notes fragments and build runs.
- Backports from `main` to the branch: commits on `main` since the `.0` tag. A row shows ✅ when the commit is on the
  branch: its backport PR (head `backport-<PR>-to-<branch>`) merged, `git cherry` finds the same patch, a branch commit
  has the same PR number, or a `[Backport ...]` commit has the same title. Backport PR status is appended when one
  exists.
- Forward-ports from the branch to `main`: commits on the release branch since the cut, ✅ when also on `main`.
- Row status: ✅ landed, ☑️ marked `/done`, 🟨 backport PR open, ⬜ not ported, ⏭️ marked `/skip`. Emoji instead of
  task-list checkboxes, which anyone with edit access could tick by accident.
- Experimental branches: open `release-X.Y.Z--*` branches with their tip.
- Human notes: free text below `<!-- HUMAN NOTES BELOW -->`, preserved verbatim.

Commands are comments on the tracker issue by a user with write access to the repository; comments by others are
ignored. `<sha>` is a commit sha prefix of at least 7 characters.

| Command                  | Effect                                                                               |
| ------------------------ | ------------------------------------------------------------------------------------ |
| `/tag`                   | Tag the branch tip with the next patch and build it. Creates no GitHub release.      |
| `/tag X.Y.Z.N`           | Same with an explicit tag. Must match the branch version and be new.                 |
| `/publish X.Y.Z.N decaf` | Publish the tag to `decaf` or `mainnet`. Lines 2..n are notes for that network only. |
| `/done <sha>`            | Mark a commit ☑️ that was ported outside the backport workflow.                      |
| `/skip <sha>`            | Mark a commit ⏭️ and strike it through: deliberately not ported.                     |
| `/unmark <sha>`          | Undo `/done` or `/skip`.                                                             |
| `/backport 123`          | Open a backport PR for merged PR 123 against this release branch (`#123` works too). |

Marks are replayed from the issue's comment history, so the body can always be regenerated.

### Operator notes

A PR that needs operator action adds `doc/operator-notes/<topic>.md` (template in its `README.md`); backports carry it.
`/publish` writes them into the release body: the fragments added or edited since the previous release on that network,
across release lines. The first publish on a network starts at the previous tag of the line, for `.0` the last tag of
the previous line. Without fragments the body says `No operator action required.` Fix a wrong fragment with a PR and tag
again.

### Publishing to a network

`/publish X.Y.Z.N decaf|mainnet` makes the tag the release operators deploy on that network. It creates the GitHub
release when the tag has none, deletes and recreates it on mainnet when its title lacks `[mainnet]`, and edits it
otherwise. Every publish regenerates the "What's Changed" section (`<!-- whats-changed -->` markers) from the previous
holder on mainnet once the release has a mainnet block, else on decaf. A rollback leaves it as is. Lines 2..n are notes
for that network only, such as an upgrade deadline. Tags that are never published have no release.

- The title gets `[decaf]`, `[mainnet]` or both. The previous release on that network loses its marker and is marked
  superseded. Latest always follows mainnet.
- Refused, with a tracker comment, unless `ghcr.io/espressosystems/espresso-network/espresso-node:<tag>` exists.
- Rerun to recover from a failed run. `/publish <older tag> <network>` rolls back.
- Do not edit release titles or the bot's `<!-- ... -->` markers by hand: they record what each network runs. The one
  exception is restoring a deleted release, see below.
- Preview without writing: `just release-publish-preview X.Y.Z.N decaf`.

GitHub sends Watch > Releases alerts on release creation only. Editing a release, or flipping it to draft and back,
sends none (tested 2026-10-21). Without a separate mainnet tag, delete and recreate is the only way to alert mainnet
operators for a release decaf already created.

| Case                                                                 | GitHub release        | Alert |
| -------------------------------------------------------------------- | --------------------- | ----- |
| decaf, tag has no release                                            | created               | yes   |
| mainnet, tag has no release                                          | created               | yes   |
| mainnet, title lacks `[mainnet]` (promotion from decaf, or rollback) | deleted and recreated | yes   |
| mainnet rerun for the current holder                                 | edited                | no    |
| decaf, release exists                                                | edited                | no    |
| superseded previous holder                                           | edited                | no    |

- A recreate keeps the tag and the URL `/releases/tag/<tag>`. Reactions, uploaded assets and the release id are lost,
  and `published_at` resets.
- If the create fails after the delete, the script recreates the old release. If that also fails, the tag has no release
  and the workflow log holds the old title and body between `# old release` and `# end old release`. A plain rerun then
  recreates the release without the decaf block. Restore it first with
  `gh release create <tag> --verify-tag --latest=false --title "<logged title>" --notes-file <logged body>`, then rerun
  `/publish`.

## Process

1. Cut the branch: `just release-cut` bumps `PHASE` from the highest existing `release-X.Y.Z`; `just release-cut 0.7.0`
   is for the branch that first activates a new protocol version on any network, decaf in practice. Only a bump of one
   part is accepted: a `PHASE` bump is allowed on any existing `MAJOR.MINOR` line, so an older protocol line can still
   get a new cut after a newer one exists, while a `MINOR` or `MAJOR` bump only applies to the highest version overall.
   An optional second argument is the source ref, default `main`. The recipe pushes the branch from your machine, since
   repository rules stop the workflow token from creating `release-*` branches, then runs the Release Branch workflow,
   which tags `X.Y.Z.0`, creates the backport label and the tracker issue, and builds images. Running the workflow from
   the Actions UI works once the branch exists, with `source_ref` set to the branch tip sha.
2. Land backport PRs and fixes on the release branch. Watch the tracker checklist.
3. Comment `/tag` on the tracker after each batch worth testing. The bot replies with the tag and a link to the
   `build.yml` runs.
4. Validate the tag on devnet, or push an experimental branch for ad hoc changes.
5. Once a tag is validated, comment `/publish X.Y.Z.N decaf`, later `/publish X.Y.Z.N mainnet`. Announce with the
   release link.
6. Delete the release branch when it is no longer needed, shipped or abandoned. The tracker is closed with a final tag
   list and labelled `release-closed`.

## Workflows

| Workflow                     | Trigger                                                      | Runs                                         |
| ---------------------------- | ------------------------------------------------------------ | -------------------------------------------- |
| `release-branch.yml`         | `workflow_dispatch`, branch `delete`                         | `scripts/release cut`, `teardown`            |
| `tag-release.yml`            | `/tag`, `/backport`, `/publish` comment, `workflow_dispatch` | `scripts/release tag`, `backport`, `publish` |
| `update-release-tracker.yml` | push to `main`/`release-*`, mark comments, dispatch          | `scripts/release refresh`                    |
| `build.yml`                  | dispatched by `cut` and `tag` for the new tag                | docker images                                |

Tags pushed by workflows do not fire `push` events, so `cut` and `tag` dispatch `build.yml` and the tracker refresh
explicitly.

Tracker refreshes run only on `main`. A run on a release branch (push, backport PR, dispatch) re-dispatches
`update-release-tracker.yml` on `main` with the original ref, so the refresh always uses `main`'s workflow and
`scripts/release` and still targets that branch's tracker. Dispatching from a feature branch also runs `main`'s copy;
test changes to `scripts/release` with the local commands below.

## Local use

All logic is in `scripts/release` (stdlib Python, tests in `scripts/test_release.py`, run with `just py::test`). It
needs `gh auth` and an `origin` remote.

```sh
just release-body 0.6.0          # render the tracker body to stdout, no writes
just release-tag release-0.6.0   # same as commenting /tag
just release-tag release-0.6.0 0.6.0.5
scripts/release tag --branch release-0.6.0 --dry-run   # show the tag, sha and tracker comment, no writes
scripts/release --help
just release-publish-preview 0.7.0.4 decaf   # preview /publish, no writes
```

`--local` makes both dry runs read the release branch from the checkout instead of `origin/*` (`main` still comes from
`origin/main`, without fetching), so a release branch and `X.Y.Z.0` tag that exist only locally can be inspected:

```sh
git branch release-0.0.1 && git tag -a 0.0.1.0 release-0.0.1 -m "Release 0.0.1.0"
scripts/release refresh --dry-run --local --version 0.0.1
scripts/release tag --dry-run --local --branch release-0.0.1
```

## Recovery

`/tag` pushes the git tag first, then dispatches `build.yml` and the tracker refresh. If a later step fails, the bot
comments the error on the tracker and the tag stays. Finish by hand rather than tagging again:

```sh
gh workflow run build.yml --ref X.Y.Z.N
```

If `just release-cut` pushed the branch but the dispatch failed, rerunning it is rejected because the branch exists.
Dispatch the workflow directly with the branch tip; `cut` skips whatever already exists:

```sh
gh workflow run release-branch.yml -f version=X.Y.Z -f source_ref=$(git rev-parse origin/release-X.Y.Z)
```

## Protection

- Tracker commands and `workflow_dispatch` require write access (`admin`, `maintain` or `write`). The workflow `if:`
  filters on `author_association` to skip runs cheaply; `scripts/release` checks the commenter's permission via the
  collaborators API, and mark replay ignores commands by users without write access.
- Trackers are only recognized when opened by `github-actions[bot]`. They are not locked: the workflow token cannot
  comment on locked issues.
- Concurrency groups are job-level, so skipped runs from outside comments or fork PRs never cancel a pending run.
- A newer pending run replaces an older one in the same group. Post tracker commands one at a time and repost one whose
  bot reply is missing.
- Tag protection rules cannot exempt the workflow token, so tags are not protected yet.
- Floating docker tags per network (`decaf`, `mainnet`) and automated promotion are not part of this process. Operators
  pin release tags.
