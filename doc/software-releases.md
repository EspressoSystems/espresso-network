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
  YYYYMMDD.rcN   ----build.yml---->  image:YYYYMMDD.rcN
  (Pre-release)                           |
       |                                  | promote + approval
       | create-release.yml               v
       v                             decaf.rc
  YYYYMMDD       ----build.yml---->  image:YYYYMMDD
  (Release)                               |
                                          | promote + approval
                                          v
                                        decaf
                                          |
                                          | promote + approval
                                          v
                                      mainnet.rc
                                          |
                                          | promote + approval
                                          v
                                       mainnet
```

## Git Tags

Date-based. `-` for internal, `.` for qualifiers.

| Git Tag        | GitHub Release | Audience                       |
| -------------- | -------------- | ------------------------------ |
| `YYYYMMDD-*`   | Pre-release    | Internal testing               |
| `YYYYMMDD.rcN` | Pre-release    | Safe to deploy, early adopters |
| `YYYYMMDD`     | Release        | Recommended for all operators  |

RC pre-releases are tested and safe to deploy. Operators willing to run canary versions are encouraged to use them and
report issues.

Multiple releases on the same day use `YYYYMMDD.1`, `YYYYMMDD.2`, etc.

Git tags are created via `create-release.yml`, which requires reviewer approval. The workflow validates the tag format,
classifies it as release or pre-release, and creates both the git tag and GitHub Release with auto-generated notes. This
triggers `build.yml`, which builds Docker images tagged with the git tag (e.g. git tag `20260408` produces Docker image
`espresso-node:20260408`).

## Docker Floating Tags

Floating Docker tags track network rollout. They are not git tags and are never created by `build.yml`.

| Docker Tag   | Points to     | Audience                 |
| ------------ | ------------- | ------------------------ |
| `decaf.rc`   | Latest RC     | Canary decaf operators   |
| `decaf`      | Latest stable | Decaf operators          |
| `mainnet.rc` | Latest RC     | Canary mainnet operators |
| `mainnet`    | Latest stable | Mainnet operators        |

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
2. Create git tag `YYYYMMDD-description` via `create-release.yml`. Creates GitHub Pre-release.
3. Optionally create git tag `YYYYMMDD.rcN` via `create-release.yml`. Creates GitHub Pre-release.
4. Create git tag `YYYYMMDD` via `create-release.yml`. Creates GitHub Release.
5. Promote the release to `decaf.rc` (requires approval).
6. After confidence on decaf canaries, promote the release to `decaf` (requires approval).
7. Promote the release to `mainnet.rc` (requires approval).
8. After confidence on mainnet canaries, promote the release to `mainnet` (requires approval).
9. Post on Discord/Telegram with release link.

### Hotfixes

For critical bugfixes that need to skip the full decaf progression, the promote action supports a `skip-progression`
flag. The `release` environment approval still applies, so a reviewer must sign off. The action's run history records
that progression was skipped.

## Create Release Action

`create-release.yml` creates git tags and GitHub Releases. Requires approval via the `release` environment.

Inputs: `tag` (e.g. `20260408`), `ref` (branch or commit to tag).

`gh workflow run create-release.yml -f tag=20260408 -f ref=release-vid-upgrade`

## Floating Tag Action

`promote-docker-tag.yml` moves floating Docker tags. Re-tags the existing Docker image (no rebuild, just a manifest
pointer). The action's run history serves as the audit trail for which release each network is running.

Inputs: `floating-tag` (one of `decaf.rc`, `decaf`, `mainnet.rc`, `mainnet`), `release-tag` (the git tag to point to).

`gh workflow run promote-docker-tag.yml -f floating-tag=decaf.rc -f release-tag=20260408`

Enforces progression: `decaf.rc` -> `decaf` -> `mainnet.rc` -> `mainnet`. Use `skip-progression` for hotfixes.

### Protection

- **Git tags**: All `YYYYMMDD*` git tags are created via `create-release.yml`, which requires approval through the
  `release` GitHub environment. Direct tag pushes should be blocked by git tag protection rules.
- **Floating Docker tags**: All promotions require approval from a reviewer via the `release` GitHub environment.
