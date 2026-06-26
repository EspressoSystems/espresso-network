#!/usr/bin/env python3
"""Maintain the release tracker issue for a release-X.Y.Z branch.

Subcommands:
    refresh   Re-render the regenerated sections (tag log, promotion state,
              experimental branches) and append new commits to the backport /
              forward-port checklists.
    tick      Tick the checkbox for one commit sha in a checklist section
              (used by backport.yml after a backport PR merges).

The body of the tracker issue is the source of truth for everything. Hidden
markers store cursor state (last sha seen on main / on the release branch)
so the checklists are append-only between runs.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys

REGISTRY = "ghcr.io/espressosystems/espresso-network"
REFERENCE_SERVICE = "espresso-node"
FLOATING_STAGES = ["decaf.canary", "decaf", "mainnet.canary", "mainnet"]
BRANCH_MARKER = re.compile(r"<!-- release-branch: (release-[0-9]+\.[0-9]+\.[0-9]+) -->")
CURSOR_MARKER = re.compile(r"<!-- (main|branch)-cursor: ([0-9a-f]{40}|none) -->")


def run(cmd: list[str], check: bool = True, input_text: str | None = None) -> str:
    r = subprocess.run(cmd, capture_output=True, text=True, input=input_text, check=False)
    if check and r.returncode != 0:
        sys.stderr.write(f"command failed: {' '.join(cmd)}\n{r.stderr}\n")
        sys.exit(r.returncode)
    return r.stdout


def get_body(issue: int) -> str:
    return run(["gh", "issue", "view", str(issue), "--json", "body", "-q", ".body"]).rstrip("\n")


def set_body(issue: int, body: str) -> None:
    run(["gh", "issue", "edit", str(issue), "--body-file", "-"], input_text=body)


def extract_branch(body: str) -> str:
    m = BRANCH_MARKER.search(body)
    if not m:
        sys.stderr.write("tracker body missing release-branch marker\n")
        sys.exit(2)
    return m.group(1)


def get_cursor(body: str, kind: str) -> str | None:
    for m in CURSOR_MARKER.finditer(body):
        if m.group(1) == kind:
            return None if m.group(2) == "none" else m.group(2)
    return None


def set_cursor(body: str, kind: str, sha: str | None) -> str:
    new = f"<!-- {kind}-cursor: {sha or 'none'} -->"
    pattern = re.compile(rf"<!-- {kind}-cursor: [0-9a-f]+|<!-- {kind}-cursor: none -->")
    if pattern.search(body):
        return re.sub(rf"<!-- {kind}-cursor: ([0-9a-f]+|none) -->", new, body)
    return body + "\n" + new + "\n"


def replace_section(body: str, name: str, content: str) -> str:
    pat = re.compile(rf"(<!-- BEGIN {name} -->)(.*?)(<!-- END {name} -->)", re.DOTALL)
    if not pat.search(body):
        sys.stderr.write(f"section {name} not found\n")
        return body
    return pat.sub(rf"\1\n{content}\n\3", body)


def read_section(body: str, name: str) -> str:
    m = re.search(rf"<!-- BEGIN {name} -->\n(.*?)\n<!-- END {name} -->", body, re.DOTALL)
    return m.group(1) if m else ""


# --- Sections regenerated every refresh -------------------------------------


def render_tag_log(version: str) -> str:
    out = run([
        "git", "tag", "--list", f"{version}.*",
        "--sort=-creatordate",
        "--format=%(refname:short)\t%(creatordate:short)\t%(objectname:short)",
    ]).strip()
    if not out:
        return "## Tag log\n\n_No tags yet._"
    lines = ["## Tag log", "", "| Tag | Date | Commit |", "|---|---|---|"]
    for row in out.splitlines():
        parts = row.split("\t")
        if len(parts) >= 3:
            lines.append(f"| `{parts[0]}` | {parts[1]} | `{parts[2][:8]}` |")
    return "\n".join(lines)


def get_digest(tag: str) -> str | None:
    r = subprocess.run(
        ["docker", "buildx", "imagetools", "inspect", "--raw",
         f"{REGISTRY}/{REFERENCE_SERVICE}:{tag}"],
        capture_output=True, text=True, check=False,
    )
    if r.returncode != 0 or not r.stdout.strip():
        return None
    try:
        manifest = json.loads(r.stdout)
    except json.JSONDecodeError:
        return None
    for m in manifest.get("manifests", []):
        plat = m.get("platform", {})
        if plat.get("architecture") == "amd64" and plat.get("os") == "linux":
            return m.get("digest")
    return None


def render_promotion_state(version: str) -> str:
    branch_tags = run(["git", "tag", "--list", f"{version}.*", "--sort=-creatordate"]).split()
    digest_to_tag: dict[str, str] = {}
    for t in branch_tags:
        d = get_digest(t)
        if d and d not in digest_to_tag:
            digest_to_tag[d] = t
    lines = ["## Promotion state", "", "| Stage | Tag |", "|---|---|"]
    for stage in FLOATING_STAGES:
        d = get_digest(stage)
        if d is None:
            lines.append(f"| `{stage}` | _none_ |")
        elif d in digest_to_tag:
            lines.append(f"| `{stage}` | `{digest_to_tag[d]}` |")
        else:
            lines.append(f"| `{stage}` | _(unrelated tag)_ |")
    return "\n".join(lines)


def render_experimental_branches(version: str, repo: str) -> str:
    prefix = f"release-{version}/"
    out = run([
        "gh", "api", f"repos/{repo}/branches", "--paginate", "-q",
        f'.[] | select(.name | startswith("{prefix}")) | "\\(.name)\\t\\(.commit.sha)"',
    ]).strip()
    if not out:
        return "## Experimental branches\n\n_None._"
    lines = ["## Experimental branches", "", "| Branch | Last commit |", "|---|---|"]
    for row in out.splitlines():
        parts = row.split("\t")
        if len(parts) >= 2:
            lines.append(f"| [`{parts[0]}`](https://github.com/{repo}/tree/{parts[0]}) | `{parts[1][:8]}` |")
    return "\n".join(lines)


# --- Append-only checklists --------------------------------------------------


def append_commits(body: str, section: str, title: str, cursor_kind: str, rev: str) -> str:
    """Append new commits on `rev` since the last cursor to `section`.

    If the cursor is invalid (force-push, branch rewrite), reset and append a
    warning line.
    """
    cursor = get_cursor(body, cursor_kind)
    head = run(["git", "rev-parse", rev]).strip()
    if not head:
        return body

    reset = False
    if cursor:
        is_ancestor = subprocess.run(
            ["git", "merge-base", "--is-ancestor", cursor, head], check=False
        )
        if is_ancestor.returncode != 0:
            reset = True

    if cursor is None or reset:
        # First run, or branch was rewritten. Just record HEAD and don't dump
        # the full history into the checklist.
        new_lines = []
        if reset:
            new_lines.append(f"_Branch rewrite detected; checklist resumed from `{head[:8]}`._")
    else:
        log = run([
            "git", "log", f"{cursor}..{head}",
            "--reverse",
            "--no-merges",
            "--pretty=format:%H\t%h\t%s",
        ]).strip()
        new_lines = []
        for row in log.splitlines():
            parts = row.split("\t", 2)
            if len(parts) == 3:
                full, short, subject = parts
                # Escape subject pipes for markdown safety.
                subject = subject.replace("|", "\\|")
                new_lines.append(f"- [ ] `{short}` {subject} <!-- sha: {full} -->")

    existing = read_section(body, section)
    # Strip the placeholder template body, but keep prior bot output.
    placeholder_lines = {
        "Commits on `main` not yet in this release branch.",
        "Commits on this release branch not yet in `main`.",
        "_Updated by the tracker bot on push events._",
    }
    kept = []
    for line in existing.splitlines():
        s = line.strip()
        if s in placeholder_lines or s == "" or s.startswith(f"## {title}"):
            continue
        kept.append(line)
    combined = kept + new_lines
    if not combined:
        rendered = f"## {title}\n\n_None yet._"
    else:
        rendered = f"## {title}\n\n" + "\n".join(combined)
    body = replace_section(body, section, rendered)
    body = set_cursor(body, cursor_kind, head)
    return body


def tick_box(body: str, section: str, sha_or_short: str) -> str:
    """Tick the checkbox in `section` for the entry whose hidden sha matches."""
    existing = read_section(body, section)
    target = sha_or_short.lower()

    def repl(line: str) -> str:
        m = re.match(r"^- \[ \] `([0-9a-f]+)` (.*?) <!-- sha: ([0-9a-f]{40}) -->\s*$", line)
        if not m:
            return line
        full = m.group(3)
        if full.startswith(target) or target.startswith(m.group(1)):
            return f"- [x] `{m.group(1)}` {m.group(2)} <!-- sha: {full} -->"
        return line

    updated = "\n".join(repl(l) for l in existing.splitlines())
    # Determine the human title from the existing first line if available.
    title_line = next((l for l in existing.splitlines() if l.startswith("## ")), None)
    if title_line:
        # already includes the title in `updated`
        rendered = updated
    else:
        rendered = updated
    return replace_section(body, section, rendered)


# --- Commands ----------------------------------------------------------------


def cmd_refresh(args: argparse.Namespace) -> None:
    repo = os.environ["GH_REPO"]
    body = get_body(args.issue)
    branch = extract_branch(body)
    version = branch[len("release-"):]

    run(["git", "fetch", "origin", "main", branch], check=False)

    body = replace_section(body, "TAG_LOG", render_tag_log(version))
    body = replace_section(body, "PROMOTION_STATE", render_promotion_state(version))
    body = replace_section(body, "EXPERIMENTAL_BRANCHES",
                           render_experimental_branches(version, repo))
    body = append_commits(
        body, "BACKPORT_CANDIDATES", "Backport candidates",
        "main", "origin/main",
    )
    body = append_commits(
        body, "FORWARD_PORT_CANDIDATES", "Forward-port candidates",
        "branch", f"origin/{branch}",
    )
    set_body(args.issue, body)


def cmd_tick(args: argparse.Namespace) -> None:
    body = get_body(args.issue)
    body = tick_box(body, args.section, args.sha)
    set_body(args.issue, body)


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument("--issue", type=int, required=True)
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("refresh")

    t = sub.add_parser("tick")
    t.add_argument("--section", required=True,
                   choices=["BACKPORT_CANDIDATES", "FORWARD_PORT_CANDIDATES"])
    t.add_argument("--sha", required=True)

    args = p.parse_args()
    {"refresh": cmd_refresh, "tick": cmd_tick}[args.cmd](args)


if __name__ == "__main__":
    main()
