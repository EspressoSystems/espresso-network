---
name: Release Tracker
about: Auto-created by setup-release-branch.yml when a release-X.Y.Z branch is created. Not for manual use.
title: "Release X.Y.Z"
labels: release-tracker
---

<!-- release-branch: release-__VERSION__ -->

Tracker for release branch [`release-__VERSION__`](https://github.com/espressosystems/espresso-network/tree/release-__VERSION__).

Run release commands as comments on this issue (see [RELEASES.md](https://github.com/espressosystems/espresso-network/blob/main/RELEASES.md)):

- `/tag` — cut the next patch tag
- `/promote <stage>` — promote the latest tag (one of `decaf.rc`, `decaf`, `mainnet.rc`, `mainnet`)
- `/skip <sha> [reason]` / `/unskip <sha>` — manage backport / forward-port candidate lists

Anything you write outside the `<!-- BEGIN ... -->` / `<!-- END ... -->` markers is preserved across automated updates.

<!-- BEGIN TAG_LOG -->
## Tag log

_No tags yet._
<!-- END TAG_LOG -->

<!-- BEGIN PROMOTION_STATE -->
## Promotion state

| Stage | Tag |
|---|---|
| `decaf.rc` | _none_ |
| `decaf` | _none_ |
| `mainnet.rc` | _none_ |
| `mainnet` | _none_ |
<!-- END PROMOTION_STATE -->

<!-- BEGIN BACKPORT_CANDIDATES -->
## Backport candidates

Commits on `main` not yet in this release branch.

_Updated by the tracker bot on push events._
<!-- END BACKPORT_CANDIDATES -->

<!-- BEGIN FORWARD_PORT_CANDIDATES -->
## Forward-port candidates

Commits on this release branch not yet in `main`.

_Updated by the tracker bot on push events._
<!-- END FORWARD_PORT_CANDIDATES -->

<!-- BEGIN SKIP_LIST -->
## Skip list

Commits explicitly excluded from the candidate lists above. Managed by `/skip <sha> [reason]` and `/unskip <sha>` comments.

_Empty._
<!-- END SKIP_LIST -->

<!-- BEGIN EXPERIMENTAL_BRANCHES -->
## Experimental branches

Currently-open branches matching `release-__VERSION__/<topic>`.

_Updated by the tracker bot on push events._
<!-- END EXPERIMENTAL_BRANCHES -->

## Notes

_Use this space for human notes; it is preserved across automated updates._
