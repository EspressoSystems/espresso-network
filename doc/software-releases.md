# Release Versioning

## Tags

### Git Tags

Date-based. `-` for internal, `.` for qualifiers.

| Tag            | GitHub Release | Audience                       |
| -------------- | -------------- | ------------------------------ |
| `YYYYMMDD-*`   | Pre-release    | Internal testing               |
| `YYYYMMDD.rcN` | Pre-release    | Safe to deploy, early adopters |
| `YYYYMMDD`     | Release        | Recommended for all operators  |

RC pre-releases are tested and safe to deploy. Operators willing to run canary versions are encouraged to use them and
report issues.

### Docker Floating Tags

Git tags track the software version. Floating Docker tags track network rollout. These are separate concerns.

| Floating Tag | Points to     | Audience                 |
| ------------ | ------------- | ------------------------ |
| `decaf.rc`   | Latest RC     | Canary decaf operators   |
| `decaf`      | Latest stable | Decaf operators          |
| `mainnet.rc` | Latest RC     | Canary mainnet operators |
| `mainnet`    | Latest stable | Mainnet operators        |

- Each release also produces an image tagged with its git tag (e.g. `20260408`, `20260408.rc1`).
- Floating tags are moved via a manually triggered GitHub Action (see below).
- Operator docs reference floating tags so they don't need updating on new releases.
- Operators who prefer pinned versions use the date tag and watch GitHub Releases.

## Branches

Release branches start with `release-`.

- Branch off main.
- Fixes from testing go on the release branch.
- Backports from main are cherry-picked: if possible with existing backport action, otherwise manually (use
  `cherry-pick -x`).

## Process

1. Create `release-*` branch off main. Test and fixup.
2. Tag `YYYYMMDD-description` for internal testing. GitHub Pre-release.
3. Optionally tag `YYYYMMDD.rcN` for release candidates. GitHub Pre-release.
4. Tag `YYYYMMDD`. GitHub Release.
5. Move `decaf.rc` Docker tag.
6. After confidence on decaf canaries, move `decaf` Docker tag.
7. Move `mainnet.rc` Docker tag.
8. After confidence on mainnet canaries, move `mainnet` Docker tag.
9. Post on Discord/Telegram with release link.

## Floating Tag Action

A `workflow_dispatch` GitHub Action to move floating Docker tags. Inputs:

- **floating-tag**: one of `decaf.rc`, `decaf`, `mainnet.rc`, `mainnet`
- **release-tag**: the git tag to point to (e.g. `20260408` or `20260408.rc1`)

The action pulls the image by its git tag and pushes it under the floating tag name. Floating tags exist only in the
Docker registry, not as git tags. This avoids force-pushing git tags and avoids re-triggering `build.yml`. The action's
run history serves as the audit trail for which release each network is running.

### Protection

The `build.yml` workflow triggers on any `YYYYMMDD*` git tag push and builds a Docker image. An accidental or
unauthorized tag push would produce a Docker image that the floating tag action could then point to.

- **Git tag protection rules**: Restrict who can push `YYYYMMDD` and `YYYYMMDD.rcN` tags. Internal testing tags
  (`YYYYMMDD-*`) can remain unrestricted.
- **Floating tag action**: Requires environment approval (GitHub environment protection rules) so moving `mainnet` or
  `mainnet.rc` needs explicit sign-off.

## Auto-Generated Release Notes

Configure `.github/release.yml` to categorize PRs by label. See
[GitHub docs](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes).
