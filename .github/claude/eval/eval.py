"""Replay the Claude PR review on PRs with known bugs; write a report to score by hand.

    just claude::eval [PR ...] [--model opus] [--jobs 10] [--prepare-only] [--out DIR]

Per PR: check out the head in a worktree under tmp/claude-eval/wt/<pr>, write a minimal
pull_request event and an empty comments file, and run the `review` job in-process with
`claude_gh.amain`. Worktrees are prepared one after another, then up to `--jobs` reviews
run concurrently. Results go to <out>/<pr>/ and <out>/report.md.

Never writes to GitHub: only `run` is called, with a GitHub whose writes raise.

Env: GH_TOKEN (optional; else `gh auth token`; else unauthenticated), ANTHROPIC_API_KEY,
CLAUDE_CONFIG_DIR, CLAUDE_CLI_PATH (all optional).
"""

import argparse
import asyncio
import contextvars
import json
import logging
import os
import re
import shutil
import subprocess
import sys
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from githubkit.exception import RequestFailed

import claude_gh as cg

REPO = Path(__file__).resolve().parents[3]
FULL_NAME = "EspressoSystems/espresso-network"
CLONE_URL = f"https://github.com/{FULL_NAME}.git"
CASES = Path(__file__).with_name("cases.toml")
ROOT = REPO / "tmp" / "claude-eval"
WORKTREES = ROOT / "wt"
# Shape of scripts/fetch-pr-team-comments. Empty so later discussion of the bug cannot leak in.
EMPTY_COMMENTS = {"comments": [], "reviewThreads": []}
CURRENT_PR: contextvars.ContextVar[int | None] = contextvars.ContextVar(
    "current_pr", default=None
)
log = logging.getLogger("claude-eval")
STALE = ("actions.json", "actions.json.part", "is-backport", "pr.diff")


def main() -> None:
    cases = load_cases()
    args = parse_args(cases)
    out = args.out or ROOT / f"{datetime.now(UTC):%Y%m%dT%H%M%SZ}-{args.model}"
    out.mkdir(parents=True)  # FileExistsError: never overwrite an earlier run
    logging.getLogger().setLevel(logging.INFO)
    prs = args.pr or list(cases)
    asyncio.run(evaluate(prs, args.model, args.prepare_only, args.jobs, out))
    if not args.prepare_only:
        report = "\n".join(section(pr, cases[pr], out / str(pr)) for pr in prs)
        (out / "report.md").write_text(f"# Review eval, model {args.model}\n\n{report}")
        print(f"report: {out / 'report.md'}")
        if raised := [pr for pr in prs if exit_code(out / str(pr)).startswith("error")]:
            raise SystemExit(f"cases raised: {raised}, see their log.txt")


async def evaluate(
    prs: list[int], model: str, prepare_only: bool, jobs: int, out: Path
) -> None:
    token = github_token()
    kit = cg.GitHubKit(FULL_NAME, token)
    gh = ReadOnlyGitHub(kit)
    worktrees = {}
    for i, pr in enumerate(prs, 1):
        print(f"[{i}/{len(prs)}] {pr}: preparing", flush=True)
        worktrees[pr] = prepare(pr, await fetch_pull(kit, pr), out / str(pr))
    if prepare_only:
        return
    slots = asyncio.Semaphore(jobs)

    async def run(pr: int, wt: Path) -> None:
        async with slots:
            print(f"{pr}: reviewing with {model}", flush=True)
            CURRENT_PR.set(pr)
            await review(gh, token, model, wt, out / str(pr), pr)
            print(result_line(pr, out / str(pr)), flush=True)

    async with asyncio.TaskGroup() as tg:
        for pr, wt in worktrees.items():
            tg.create_task(run(pr, wt))


def load_cases() -> dict[int, dict[str, Any]]:
    return {c["pr"]: c for c in tomllib.loads(CASES.read_text())["case"]}


def parse_args(cases: dict[int, dict[str, Any]]) -> argparse.Namespace:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument("pr", nargs="*", type=int, help="default: every case")
    ap.add_argument("--model", default="opus")
    ap.add_argument("--jobs", type=positive, default=10, help="concurrent reviews")
    ap.add_argument("--prepare-only", action="store_true", help="no Claude run")
    ap.add_argument(
        "--out", type=Path, help="default: tmp/claude-eval/<UTC time>-<model>"
    )
    args = ap.parse_args()
    if unknown := [pr for pr in args.pr if pr not in cases]:
        ap.error(f"not in {CASES.name}: {unknown}")
    return args


def positive(value: str) -> int:
    if (n := int(value)) < 1:
        raise argparse.ArgumentTypeError(f"must be at least 1, got {n}")
    return n


def github_token() -> str | None:
    if token := os.environ.get("GH_TOKEN"):
        return token
    if shutil.which("gh"):
        res = subprocess.run(
            ["gh", "auth", "token"], capture_output=True, text=True, check=False
        )
        if res.returncode == 0:
            return res.stdout.strip()
    print("no GitHub token: unauthenticated, 60 requests per hour", file=sys.stderr)
    return None


class ReadOnly(RuntimeError):
    pass


class ReadOnlyGitHub:
    """`cg.GitHub` with GET reads delegated to `inner`; every write raises."""

    def __init__(self, inner: cg.GitHub):
        self.inner = inner

    async def pr_diff(self, pr: int) -> str:
        return await self.inner.pr_diff(pr)

    async def compare_diff(self, base: str, head: str) -> str:
        return await self.inner.compare_diff(base, head)

    async def pr_view(self, pr: int) -> str:
        return await self.inner.pr_view(pr)

    async def issue_view(self, number: int) -> str:
        return await self.inner.issue_view(number)

    async def head_sha(self, pr: int) -> str:
        return await self.inner.head_sha(pr)

    async def pr_payload(self, pr: int) -> dict:
        return await self.inner.pr_payload(pr)

    async def thread(self, thread_id: str) -> cg.Thread:
        raise ReadOnly("thread: GraphQL, only apply needs it")

    async def inline_comment(
        self, pr: int, path: str, line: int, body: str, sha: str
    ) -> None:
        raise ReadOnly("inline_comment")

    async def reply(self, pr: int, comment_id: int, body: str) -> cg.Posted:
        raise ReadOnly("reply")

    async def comment(self, number: int, body: str) -> cg.Posted:
        raise ReadOnly("comment")

    async def edit_comment(self, comment: cg.Posted, body: str) -> None:
        raise ReadOnly("edit_comment")

    async def resolve_thread(self, thread_id: str) -> None:
        raise ReadOnly("resolve_thread")


async def fetch_pull(kit: cg.GitHubKit, pr: int) -> dict[str, Any]:
    try:
        resp = await kit.gh.arequest("GET", f"/repos/{FULL_NAME}/pulls/{pr}")
    except RequestFailed as e:
        if e.response.status_code in (403, 429):
            raise SystemExit(
                f"GitHub refused PR {pr} ({e.response.status_code}), likely the rate"
                " limit: set GH_TOKEN or run `gh auth login`"
            ) from e
        raise
    return resp.json()


def prepare(pr: int, data: dict[str, Any], pr_out: Path) -> Path:
    """Check out the PR head, write the event to `pr_out`, reset the worktree's tmp/."""
    wt = checkout(pr, data["head"]["sha"])
    pr_out.mkdir()
    (pr_out / "event.json").write_text(json.dumps(event(pr, data), indent=2))
    tmp = wt / "tmp"
    tmp.mkdir(exist_ok=True)
    for name in STALE:
        (tmp / name).unlink(missing_ok=True)
    (tmp / "pr-review-comments.json").write_text(json.dumps(EMPTY_COMMENTS))
    return wt


def checkout(pr: int, sha: str) -> Path:
    wt = WORKTREES / str(pr)
    if git("cat-file", "-e", f"{sha}^{{commit}}", check=False).returncode:
        git("fetch", "--quiet", CLONE_URL, f"pull/{pr}/head")
    if wt.exists():
        git("-C", str(wt), "checkout", "--quiet", "--detach", sha)
    else:
        git("worktree", "add", "--quiet", "--detach", str(wt), sha)
    return wt


def git(*args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args], cwd=REPO, check=check, text=True, stderr=subprocess.PIPE
    )


def event(pr: int, data: dict[str, Any]) -> dict[str, Any]:
    keep = ("sha", "ref")
    return {
        "action": "opened",
        "number": pr,
        "pull_request": {
            "number": pr,
            "title": data["title"],
            "body": data["body"],
            "user": {"login": data["user"]["login"]},
            "head": {k: data["head"][k] for k in keep},
            "base": {k: data["base"][k] for k in keep},
        },
        "repository": {"full_name": FULL_NAME},
    }


async def review(
    gh: ReadOnlyGitHub,
    token: str | None,
    model: str,
    wt: Path,
    pr_out: Path,
    pr: int,
) -> None:
    """Run the review job with Claude's cwd in `wt`; copy log, exit code and record."""
    args = cg.parse_args(["run", "review", "--model", model, "--workdir", str(wt)])
    env = {
        "GITHUB_EVENT_PATH": str(pr_out / "event.json"),
        "GITHUB_EVENT_NAME": "pull_request",
        "GITHUB_REPOSITORY": FULL_NAME,
        "GITHUB_SERVER_URL": "https://github.com",
        "GITHUB_RUN_ID": "0",
        "GH_TOKEN": token or "",
    }
    if key := os.environ.get(
        "ANTHROPIC_API_KEY"
    ):  # unset: the CLI uses the local login
        env["ANTHROPIC_API_KEY"] = key
    handler = logging.FileHandler(pr_out / "log.txt")
    handler.addFilter(lambda _: CURRENT_PR.get() == pr)
    handler.setFormatter(
        logging.Formatter("%(asctime)s %(levelname)s %(name)s: %(message)s")
    )
    root = logging.getLogger()
    root.addHandler(handler)
    try:
        code = await cg.amain(args, gh, env, cg.sdk_runner, wt / "tmp")
    except Exception as e:  # one broken case must not cancel the others
        log.exception("review raised")
        code = f"error: {e!r}"
    finally:
        root.removeHandler(handler)
        handler.close()
    (pr_out / "exit").write_text(str(code))
    record = wt / "tmp" / "actions.json"
    if record.exists():
        shutil.copy(record, pr_out / "actions.json")


def exit_code(pr_out: Path) -> str:
    return (pr_out / "exit").read_text().strip()


def load_record(pr_out: Path) -> cg.Record | None:
    path = pr_out / "actions.json"
    return (
        cg.parse_record(json.loads(path.read_text()), "review")
        if path.exists()
        else None
    )


def footer(record: cg.Record) -> str:
    text = record.footer or "no result"
    return text.removeprefix("<sub>").removesuffix("</sub>")


FINDING = re.compile(r"- \*\*(blocking|should fix|concern|nit)\*\*:")


def findings(record: cg.Record) -> list[str]:
    return [line for line in (record.summary or "").splitlines() if FINDING.match(line)]


def wrong(record: cg.Record) -> list[cg.Candidate]:
    return [c for c in record.candidates if c.status == "wrong"]


def unjudged(record: cg.Record) -> list[cg.Candidate]:
    return [c for c in record.candidates if c.status is None]


def result_line(pr: int, pr_out: Path) -> str:
    code = exit_code(pr_out)
    record = load_record(pr_out)
    if record is None:
        return f"{pr}: exit={code} no actions.json"
    return (
        f"{pr}: exit={code} {footer(record)} findings={len(findings(record))}"
        f" wrong={len(wrong(record))} unjudged={len(unjudged(record))}"
        f" error={record.error}"
    )


def section(pr: int, case: dict[str, Any], pr_out: Path) -> str:
    fixes = ", ".join(f"#{n}" for n in case["fixed_by"])
    head = f"## #{pr} (fixed by {fixes})\n\nBug: {case['bug']}\n\n"
    record = load_record(pr_out)
    if record is None:
        return (
            head
            + f"No record, exit {(pr_out / 'exit').read_text().strip()}: see log.txt.\n"
        )
    lines = [
        f"{footer(record)} · exit {(pr_out / 'exit').read_text().strip()} · error {record.error}"
    ]
    lines += [
        "",
        "Findings:",
        *(findings(record) or ["- none"]),
        "",
        "Wrong candidates:",
        *candidate_lines(wrong(record), evidence=True),
        "",
        "Unjudged candidates:",
        *candidate_lines(unjudged(record), evidence=False),
    ]
    return head + "\n".join(lines) + "\n"


def candidate_lines(candidates: list[cg.Candidate], evidence: bool) -> list[str]:
    lines = []
    for c in candidates:
        lines.append(f"- {c.path}:{c.line} {cg.one_line(c.summary, 300)}")
        if evidence:
            lines.append(f"  - evidence: {cg.one_line(c.evidence or '', 600)}")
    return lines or ["- none"]


if __name__ == "__main__":
    main()
