---
name: Release Tracker
about: Auto-created by setup-release-branch.yml when a release-X.Y.Z branch is created. Not for manual use.
title: "Release X.Y.Z"
labels: release-tracker
---

<!-- release-branch: release-__VERSION__ -->
<!-- main-cursor: none -->
<!-- branch-cursor: none -->

Tracker for release branch [`release-__VERSION__`](https://github.com/espressosystems/espresso-network/tree/release-__VERSION__).

Run release commands as comments on this issue (see [RELEASES.md](https://github.com/espressosystems/espresso-network/blob/main/RELEASES.md)):

- `/tag` — cut the next patch tag
- `/promote <stage>` — promote the latest tag (one of `decaf.canary`, `decaf`, `mainnet.canary`, `mainnet`)

The Backport / Forward-port checklists are append-only. New commits land as unchecked items. Tick them by hand when handled, or let `backport.yml` tick them automatically when a backport PR lands. Anything you write outside the `<!-- BEGIN ... -->` / `<!-- END ... -->` markers is preserved across automated updates.

<!-- BEGIN TAG_LOG -->
## Tag log

_No tags yet._
<!-- END TAG_LOG -->

<!-- BEGIN PROMOTION_STATE -->
## Promotion state

| Stage | Tag |
|---|---|
| `decaf.canary` | _none_ |
| `decaf` | _none_ |
| `mainnet.canary` | _none_ |
| `mainnet` | _none_ |
<!-- END PROMOTION_STATE -->

<!-- BEGIN BACKPORT_CANDIDATES -->
## Backport candidates

_None yet._
<!-- END BACKPORT_CANDIDATES -->

<!-- BEGIN FORWARD_PORT_CANDIDATES -->
## Forward-port candidates

_None yet._
<!-- END FORWARD_PORT_CANDIDATES -->

<!-- BEGIN EXPERIMENTAL_BRANCHES -->
## Experimental branches

_None._
<!-- END EXPERIMENTAL_BRANCHES -->

## Notes

_Use this space for human notes; it is preserved across automated updates._
