#!/usr/bin/env python3
"""Maintain the release tracker issue for a release-X.Y.Z branch.

Subcommands:
    refresh         Re-render all managed sections in the tracker issue body.
    skip            Add a sha to the skip list (from a /skip comment).
    unskip          Remove a sha from the skip list (from an /unskip comment).
    promote         Dispatch promote-docker-tag.yml for /promote <stage> comments.
    post-promote    Post the result of a promotion workflow back to the tracker.

Reads GH_TOKEN / GH_REPO from environment. Uses the `gh` CLI for issue
operations and the `git` CLI for ref/tag inspection. The release branch is
read from a `<!-- release-branch: release-X.Y.Z -->` marker in the issue body.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from typing import Iterable

REGISTRY = "ghcr.io/espressosystems/espresso-network"
REFERENCE_SERVICE = "espresso-node"
FLOATING_STAGES = ["decaf.rc", "decaf", "mainnet.rc", "mainnet"]
MAX_CANDIDATES = 100
BRANCH_MARKER = re.compile(r"<!-- release-branch: (release-[0-9]+\.[0-9]+\.[0-9]+) -->")


def run(cmd: list[str], check: bool = True, input_text: str | None = None) -> str:
    result = subprocess.run(
        cmd, capture_output=True, text=True, input=input_text, check=False
    )
    if check and result.returncode != 0:
        sys.stderr.write(f"command failed: {' '.join(cmd)}\n{result.stderr}\n")
        sys.exit(result.returncode)
    return result.stdout


def gh_json(args: list[str]) -> object:
    return json.loads(run(["gh"] + args + ["--json", "body,number,title"]))


def get_issue_body(issue: int) -> str:
    out = run(["gh", "issue", "view", str(issue), "--json", "body", "-q", ".body"])
    return out.rstrip("\n")


def update_issue_body(issue: int, body: str) -> None:
    run(["gh", "issue", "edit", str(issue), "--body-file", "-"], input_text=body)


def extract_branch(body: str) -> str:
    m = BRANCH_MARKER.search(body)
    if not m:
        sys.stderr.write("tracker body missing release-branch marker\n")
        sys.exit(2)
    return m.group(1)


def replace_section(body: str, name: str, content: str) -> str:
    """Replace content between <!-- BEGIN name --> and <!-- END name --> markers."""
    pattern = re.compile(
        rf"(<!-- BEGIN {name} -->)(.*?)(<!-- END {name} -->)",
        flags=re.DOTALL,
    )
    if not pattern.search(body):
        sys.stderr.write(f"section {name} not found in body\n")
        return body
    replacement = rf"\1\n{content}\n\3"
    return pattern.sub(replacement, body)


def read_section(body: str, name: str) -> str:
    pattern = re.compile(
        rf"<!-- BEGIN {name} -->\n(.*?)\n<!-- END {name} -->",
        flags=re.DOTALL,
    )
    m = pattern.search(body)
    return m.group(1) if m else ""


# --- Section renderers --------------------------------------------------------


def render_tag_log(version: str) -> str:
    tags = run(
        [
            "git",
            "tag",
            "--list",
            f"{version}.*",
            "--sort=-creatordate",
            "--format=%(refname:short)%09%(creatordate:short)%09%(*objectname:short)%09%(objectname:short)",
        ]
    ).strip()
    if not tags:
        return "## Tag log\n\n_No tags yet._"
    lines = ["## Tag log", "", "| Tag | Date | Commit |", "|---|---|---|"]
    for row in tags.splitlines():
        parts = row.split("\t")
        tag = parts[0]
        date = parts[1] if len(parts) > 1 else ""
        # annotated tags expose the target commit via *objectname; lightweight tags via objectname
        sha = (parts[2] if len(parts) > 2 and parts[2] else parts[3] if len(parts) > 3 else "")[:8]
        lines.append(f"| `{tag}` | {date} | `{sha}` |")
    return "\n".join(lines)


def get_digest(tag: str) -> str | None:
    out = subprocess.run(
        ["docker", "buildx", "imagetools", "inspect", "--raw", f"{REGISTRY}/{REFERENCE_SERVICE}:{tag}"],
        capture_output=True,
        text=True,
        check=False,
    )
    if out.returncode != 0 or not out.stdout.strip():
        return None
    try:
        manifest = json.loads(out.stdout)
    except json.JSONDecodeError:
        return None
    for m in manifest.get("manifests", []):
        plat = m.get("platform", {})
        if plat.get("architecture") == "amd64" and plat.get("os") == "linux":
            return m.get("digest")
    return None


def render_promotion_state(version: str) -> str:
    branch_tags = run(
        ["git", "tag", "--list", f"{version}.*", "--sort=-creatordate"]
    ).split()
    digest_to_tag: dict[str, str] = {}
    for t in branch_tags:
        d = get_digest(t)
        if d and d not in digest_to_tag:
            digest_to_tag[d] = t
    lines = [
        "## Promotion state",
        "",
        "| Stage | Tag |",
        "|---|---|",
    ]
    for stage in FLOATING_STAGES:
        d = get_digest(stage)
        if d is None:
            lines.append(f"| `{stage}` | _none_ |")
        elif d in digest_to_tag:
            lines.append(f"| `{stage}` | `{digest_to_tag[d]}` |")
        else:
            lines.append(f"| `{stage}` | _(unrelated tag)_ |")
    return "\n".join(lines)


@dataclass
class Commit:
    sha: str
    short: str
    subject: str


def list_commits(rev: str, exclude: str, limit: int) -> list[Commit]:
    out = run(
        [
            "git",
            "log",
            "--merges",
            "--first-parent",
            f"-{limit}",
            rev,
            f"^{exclude}",
            "--pretty=format:%H%x09%h%x09%s",
        ],
        check=False,
    )
    commits = []
    for row in out.splitlines():
        parts = row.split("\t", 2)
        if len(parts) == 3:
            commits.append(Commit(sha=parts[0], short=parts[1], subject=parts[2]))
    return commits


def parse_skip_list(body: str) -> dict[str, str]:
    section = read_section(body, "SKIP_LIST")
    skips: dict[str, str] = {}
    for line in section.splitlines():
        m = re.match(r"^- `([0-9a-f]{6,40})` — (.+?) \(by @[^)]+, [^)]+\)$", line)
        if m:
            skips[m.group(1)] = m.group(2)
    return skips


def render_skip_list(skips: dict[str, tuple[str, str, str]]) -> str:
    if not skips:
        return "## Skip list\n\n_Empty._"
    lines = ["## Skip list", ""]
    for sha, (reason, actor, ts) in skips.items():
        lines.append(f"- `{sha}` — {reason} (by @{actor}, {ts})")
    return "\n".join(lines)


def read_skips_full(body: str) -> dict[str, tuple[str, str, str]]:
    section = read_section(body, "SKIP_LIST")
    skips: dict[str, tuple[str, str, str]] = {}
    for line in section.splitlines():
        m = re.match(r"^- `([0-9a-f]{6,40})` — (.+?) \(by @([^,]+), ([^)]+)\)$", line)
        if m:
            skips[m.group(1)] = (m.group(2), m.group(3), m.group(4))
    return skips


def render_candidates(title: str, commits: list[Commit], skipped: set[str]) -> str:
    visible = [c for c in commits if not any(c.sha.startswith(s) or s.startswith(c.short) for s in skipped)]
    if not visible:
        return f"## {title}\n\n_None._"
    lines = [f"## {title}", ""]
    for c in visible:
        lines.append(f"- [ ] `{c.short}` {c.subject}")
    return "\n".join(lines)


def render_experimental_branches(version: str, repo: str) -> str:
    # Use the GitHub API to list remote branches; matches release-X.Y.Z/* exactly.
    pattern = f"release-{version}/"
    out = run(
        ["gh", "api", f"repos/{repo}/branches", "--paginate", "-q",
         f'.[] | select(.name | startswith("{pattern}")) | "\\(.name)\\t\\(.commit.sha)"']
    ).strip()
    if not out:
        return f"## Experimental branches\n\n_None._"
    lines = [
        "## Experimental branches",
        "",
        "| Branch | Last commit |",
        "|---|---|",
    ]
    for row in out.splitlines():
        parts = row.split("\t")
        if len(parts) >= 2:
            name = parts[0]
            sha = parts[1][:8]
            lines.append(f"| [`{name}`](https://github.com/{repo}/tree/{name}) | `{sha}` |")
    return "\n".join(lines)


# --- Commands ---------------------------------------------------------------


def cmd_refresh(args: argparse.Namespace) -> None:
    repo = os.environ["GH_REPO"]
    body = get_issue_body(args.issue)
    branch = extract_branch(body)
    version = branch[len("release-"):]

    # Make sure we have the branch and main locally for git log queries.
    run(["git", "fetch", "origin", "main", branch], check=False)

    skips = read_skips_full(body)
    skipped_shas = set(skips.keys())

    body = replace_section(body, "TAG_LOG", render_tag_log(version))
    body = replace_section(body, "PROMOTION_STATE", render_promotion_state(version))
    body = replace_section(
        body,
        "BACKPORT_CANDIDATES",
        render_candidates(
            "Backport candidates",
            list_commits(f"origin/main", f"origin/{branch}", MAX_CANDIDATES),
            skipped_shas,
        ),
    )
    body = replace_section(
        body,
        "FORWARD_PORT_CANDIDATES",
        render_candidates(
            "Forward-port candidates",
            list_commits(f"origin/{branch}", "origin/main", MAX_CANDIDATES),
            skipped_shas,
        ),
    )
    body = replace_section(body, "EXPERIMENTAL_BRANCHES", render_experimental_branches(version, repo))

    update_issue_body(args.issue, body)


def cmd_skip(args: argparse.Namespace) -> None:
    body = get_issue_body(args.issue)
    skips = read_skips_full(body)
    sha = args.sha.lower()
    if not re.match(r"^[0-9a-f]{6,40}$", sha):
        sys.stderr.write(f"bad sha: {sha}\n")
        sys.exit(2)
    skips[sha] = (args.reason or "(no reason)", args.actor, args.timestamp)
    body = replace_section(body, "SKIP_LIST", render_skip_list(skips))
    update_issue_body(args.issue, body)
    # The candidate sections still display the skipped sha until the next
    # refresh; trigger one inline so the user sees immediate feedback.
    cmd_refresh(args)


def cmd_unskip(args: argparse.Namespace) -> None:
    body = get_issue_body(args.issue)
    skips = read_skips_full(body)
    sha = args.sha.lower()
    if sha not in skips:
        # also try short-prefix match
        match = next((k for k in skips if k.startswith(sha) or sha.startswith(k)), None)
        if match is None:
            sys.stderr.write(f"sha {sha} not in skip list\n")
            return
        sha = match
    skips.pop(sha)
    body = replace_section(body, "SKIP_LIST", render_skip_list(skips))
    update_issue_body(args.issue, body)
    cmd_refresh(args)


def cmd_promote(args: argparse.Namespace) -> None:
    stage = args.stage
    if stage not in FLOATING_STAGES:
        sys.stderr.write(f"unknown stage: {stage}\n")
        sys.exit(2)
    body = get_issue_body(args.issue)
    branch = extract_branch(body)
    version = branch[len("release-"):]
    # Latest tag by creatordate
    tags = run(
        ["git", "tag", "--list", f"{version}.*", "--sort=-creatordate"]
    ).split()
    if not tags:
        run(
            ["gh", "issue", "comment", str(args.issue), "--body",
             f"@{args.actor} no tags on `{branch}` yet — run `/tag` before promoting."]
        )
        return
    tag = tags[0]
    repo = os.environ["GH_REPO"]
    run([
        "gh", "workflow", "run", "promote-docker-tag.yml",
        "-f", f"floating-tag={stage}",
        "-f", f"release-tag={tag}",
    ])
    run([
        "gh", "issue", "comment", str(args.issue), "--body",
        f"@{args.actor} dispatched promotion of `{tag}` → `{stage}`. "
        f"Approve at https://github.com/{repo}/actions/workflows/promote-docker-tag.yml."
    ])


def cmd_post_promote(args: argparse.Namespace) -> None:
    body = get_issue_body(args.issue)
    body = replace_section(body, "PROMOTION_STATE", render_promotion_state(extract_branch(body)[len("release-"):]))
    update_issue_body(args.issue, body)
    run([
        "gh", "issue", "comment", str(args.issue), "--body",
        f"Promotion of `{args.tag}` → `{args.stage}` finished: **{args.conclusion}**. "
        f"Run: {args.run_url}"
    ])


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--issue", type=int, required=True)
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("refresh")

    s = sub.add_parser("skip")
    s.add_argument("--sha", required=True)
    s.add_argument("--reason", default=None)
    s.add_argument("--actor", required=True)
    s.add_argument("--timestamp", required=True)

    s = sub.add_parser("unskip")
    s.add_argument("--sha", required=True)

    s = sub.add_parser("promote")
    s.add_argument("--stage", required=True)
    s.add_argument("--actor", required=True)

    s = sub.add_parser("post-promote")
    s.add_argument("--stage", required=True)
    s.add_argument("--tag", required=True)
    s.add_argument("--conclusion", required=True)
    s.add_argument("--run-url", required=True)

    args = p.parse_args()
    handlers = {
        "refresh": cmd_refresh,
        "skip": cmd_skip,
        "unskip": cmd_unskip,
        "promote": cmd_promote,
        "post-promote": cmd_post_promote,
    }
    handlers[args.cmd](args)


if __name__ == "__main__":
    main()
