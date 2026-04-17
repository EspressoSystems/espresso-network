<!-- STATUS: draft, in testing -->

# Release Versioning

```
  Git Tags                            Docker
  --------                            ------

  release branch
       |
       | create-release.yml
       v
  YYYYMMDD-desc  ----build.yml---->  image:YYYYMMDD-desc
  (Pre-release)
       |
       | create-release.yml
       v
  YYYYMMDD       ----build.yml---->  image:YYYYMMDD
  (Release)                               |
                                          | promote + approval
                                          v
                                      decaf.canary
                                          |
                                          | promote + approval
                                          v
                                        decaf
                                          |
                                          | promote + approval
                                          v
                                      mainnet.canary
                                          |
                                          | promote + approval
                                          v
                                       mainnet
```

## Git Tags

Date-based. `-` for internal, `.` for same-day qualifier.

| Git Tag      | GitHub Release | Audience                      |
| ------------ | -------------- | ----------------------------- |
| `YYYYMMDD-*` | Pre-release    | Internal testing              |
| `YYYYMMDD`   | Release        | Recommended for all operators |

Each version is defined once. If a version is broken mid-rollout, create a new version rather than retagging the same
commit. Multiple releases on the same day use `YYYYMMDD.1`, `YYYYMMDD.2`, etc. (these are also classified as Release).

Git tags are created via `create-release.yml`, which requires reviewer approval. The workflow validates the tag format,
classifies it as Release or Pre-release, and creates both the git tag and GitHub Release with auto-generated notes. This
triggers `build.yml`, which builds Docker images tagged with the git tag (e.g. git tag `20260408` produces Docker image
`espresso-node:20260408`).

## Docker Floating Tags

Floating Docker tags track network rollout. They are not git tags and are never created by `build.yml`.

| Docker Tag       | Points to     | Audience                 |
| ---------------- | ------------- | ------------------------ |
| `decaf.canary`   | Latest build  | Canary decaf operators   |
| `decaf`          | Latest stable | Decaf operators          |
| `mainnet.canary` | Latest build  | Canary mainnet operators |
| `mainnet`        | Latest stable | Mainnet operators        |

Canary operators opt into getting new versions first, accepting higher risk. The canary tier is for early feedback.

Floating Docker tags are moved via `promote-docker-tag.yml` (see below).

- Operator docs reference floating Docker tags so they don't need updating on new releases.
- Operators who prefer pinned versions use the date-based Docker image tag and watch GitHub Releases.

## Branches

Release branches start with `release-`.

- Branch off main.
- Fixes from testing go on the release branch.
- Backports from main are cherry-picked: if possible with existing backport action, otherwise manually (use
  `cherry-pick -x`).

## Process

1. Create `release-*` branch off main. Test and fixup.
2. Optionally create git tag `YYYYMMDD-description` via `create-release.yml` for internal testing. Creates GitHub
   Pre-release.
3. Create git tag `YYYYMMDD` via `create-release.yml`. Creates GitHub Release.
4. Promote the release to `decaf.canary` (requires approval).
5. After confidence on decaf canaries, promote the release to `decaf` (requires approval).
6. Promote the release to `mainnet.canary` (requires approval).
7. After confidence on mainnet canaries, promote the release to `mainnet` (requires approval).
8. Post on Discord/Telegram with release link.

### Hotfixes and same-day releases

If a version is broken at any stage, create a new version (`YYYYMMDD.1`, `YYYYMMDD.2`, or next day) rather than trying
to fix it in-flight. For critical bugfixes that need to skip the full canary progression, the promote action supports a
`skip-progression` flag. The `release` environment approval still applies, so a reviewer must sign off. The action's run
history records that progression was skipped.

## Create Release Action

`create-release.yml` creates git tags and GitHub Releases. Requires approval via the `release` environment.

Inputs: `tag` (e.g. `20260408`), `ref` (branch or commit to tag).

`gh workflow run create-release.yml -f tag=20260408 -f ref=release-vid-upgrade`

After the git tag and Release are created, the workflow dispatches `build.yml` against the new tag to build Docker
images. This is needed because tag pushes made via `GITHUB_TOKEN` do not trigger other workflows, but
`workflow_dispatch` does.

## Floating Tag Action

`promote-docker-tag.yml` moves floating Docker tags. Re-tags the existing Docker image (no rebuild, just a manifest
pointer). The action's run history serves as the audit trail for which release each network is running.

Inputs: `floating-tag` (one of `decaf.canary`, `decaf`, `mainnet.canary`, `mainnet`), `release-tag` (the git tag to
point to). Only `YYYYMMDD` and `YYYYMMDD.qualifier` tags are promotable; internal `YYYYMMDD-*` tags are rejected.

`gh workflow run promote-docker-tag.yml -f floating-tag=decaf.canary -f release-tag=20260408`

Enforces progression: `decaf.canary` -> `decaf` -> `mainnet.canary` -> `mainnet`. Use `skip-progression` for hotfixes.

### Protection

- **Git tags**: All `YYYYMMDD*` git tags are created via `create-release.yml`, which requires approval through the
  `release` GitHub environment. Direct tag pushes should be blocked by git tag protection rules.
- **Floating Docker tags**: All promotions require approval from a reviewer via the `release` GitHub environment.

The convention is to not self-approve. GitHub does not enforce this, but a second set of eyes is expected.
