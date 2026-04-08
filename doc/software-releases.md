# Release Versioning

## Tags

Date-based tags. `-` for internal, `.` for qualifiers.

| Tag                | GitHub Release | Audience           |
| ------------------ | -------------- | ------------------ |
| `YYYYMMDD-*`       | Pre-release    | Internal testing   |
| `YYYYMMDD.rcN`     | Pre-release    | Release candidates |
| `YYYYMMDD.decaf`   | Release        | Decaf operators    |
| `YYYYMMDD.mainnet` | Release        | Mainnet operators  |
| `YYYYMMDD`         | Release        | Both networks      |

A release can be promoted: `20260408.decaf` ships first, then tag `20260408` on the same commit when mainnet is ready.
Multiple tags/releases on one commit is fine.

## Docker

Each release produces an image tagged with its git tag (e.g. `20260408.decaf`).

- Two floating tags: `decaf` and `mainnet`, always pointing to the latest release for that network.
- These floating tags can be maintained by github actions to watch the `YYYYMMDD{,.decaf,.mainnet}` tags.
- Operator docs reference floating tags so they don't need updating on new releases.
- Operators who prefer pinned versions use the date tag and watch GitHub Releases.

## Branches

Release branches start with `release-`, descriptive names, or a version.

- Branch off main.
- Fixes from testing go on the release branch.
- Backports from main are cherry-picked: if possible with existing backport action, otherwise manually (use
  `cherry-pick -x`).

## Process

1. Create `release-*` branch off main. Test and fixup.
2. Tag `YYYYMMDD-description` for internal testing. GitHub Pre-release.
3. Optionally tag `YYYYMMDD.rcN` for release candidates. GitHub Pre-release.
4. Tag `YYYYMMDD.decaf`, `YYYYMMDD.mainnet`, or `YYYYMMDD`. GitHub Release.
5. Update floating Docker tags. (automated)
6. Optionally post on Discord/Telegram. Informational only, not the source of truth.

## Auto-Generated Release Notes

Configure `.github/release.yml` to categorize PRs by label. See
[GitHub docs](https://docs.github.com/en/repositories/releasing-projects-on-github/automatically-generated-release-notes).
