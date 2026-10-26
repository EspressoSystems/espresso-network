r"""Run Claude on a GitHub PR or issue with a fixed set of tools, in three workflow jobs.

    claude_gh.py start review          # write token: post the status comment, print its id
    claude_gh.py run review            # read token, Claude: record actions in tmp/actions.json
    claude_gh.py run code-review --model opus --effort high --workdir pr \
        --head-sha <sha> --base-sha <sha>
    claude_gh.py apply review --status-id 1 --run-result success --record tmp/actions/actions.json

Jobs: review (PR opened, or `/review` comment), mention (@claude mention, answers in the thread), code-review.

`run` gives the model Read/Grep/Glob (plus Agent on code-review) and an in-process MCP
server `gh`. There is no Bash. Reads go to the PR or issue bound at startup. Writes are only
recorded, never sent; the record is rewritten after each one.

`apply` is best effort: each action is validated and posted independently. An action that
fails validation or that GitHub rejects is skipped and listed in the status comment under
"Not applied"; apply still exits zero. Apply exits non-zero when the run did not succeed.
Only a structurally unusable record (bad JSON, unknown kind, job mismatch) is refused as a
whole.

The record is not a trust boundary on review: the PR author controls the apply workflow
too. The checks catch model mistakes and prompt injection: a thread must be started by a
bot and belong to the same PR, the target and mention routing come from the event, and
the head sha is set by `run`, never by a tool call.

Env: GH_TOKEN, GITHUB_EVENT_PATH, GITHUB_REPOSITORY, GITHUB_SERVER_URL, GITHUB_RUN_ID,
GITHUB_EVENT_NAME, ANTHROPIC_API_KEY (optional, run).
"""

import argparse
import asyncio
import json
import logging
import os
import re
import sys
from collections.abc import AsyncIterator, Awaitable, Callable, Mapping
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Literal, Protocol, get_args

from claude_agent_sdk import (
    AssistantMessage,
    ClaudeAgentOptions,
    Message,
    ResultMessage,
    SdkMcpTool,
    TextBlock,
    ToolUseBlock,
    create_sdk_mcp_server,
    query,
    tool,
)
from claude_agent_sdk.types import (
    EffortLevel,
    McpSdkServerConfig,
    SettingSource,
    SystemPromptPreset,
)
from githubkit import GitHub as GitHubClient
from githubkit import TokenAuthStrategy, UnauthAuthStrategy
from githubkit.exception import RequestFailed

log = logging.getLogger("claude-gh")

BUILTIN = ["Read", "Grep", "Glob"]
# Deny rules need one entry per tool; this keeps /proc/*/environ (tokens) unreadable.
DISALLOWED = ["Read(//proc/**)", "Grep(//proc/**)", "Glob(//proc/**)"]
# Other workflows also post as github-actions; only bodies starting with MARKER are ours.
# Must match scripts/fetch-pr-team-comments.
MARKER = "<!-- claude-gh -->"
WRITES = {"inline_comment", "reply", "comment", "resolve_thread"}
JOB_WRITES = {
    "review": WRITES,
    "mention": {"comment"},
    "code-review": WRITES - {"resolve_thread"},
}
# The CLI gets no GitHub token: all GitHub access goes through the `gh` MCP server.
# Background subagents outlive the main result and leave the review unposted.
CLI_ENV = {"GH_TOKEN": "", "CLAUDE_CODE_DISABLE_BACKGROUND_TASKS": "1"}
MAX_TURNS = 120
SERVER = "gh"
THREAD_ID = re.compile(r"[A-Za-z0-9_=-]+")
ESCAPE = re.compile(rb"\\(?:([0-7]{3})|(.))")
C_ESCAPES = {
    b"a": b"\a",
    b"b": b"\b",
    b"f": b"\f",
    b"n": b"\n",
    b"r": b"\r",
    b"t": b"\t",
    b"v": b"\v",
    b"\\": b"\\",
    b'"': b'"',
}
HUNK = re.compile(r"@@ -\d+(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
CLAUDE_DIR = Path(__file__).parent
TMP = Path("tmp")
READS = {"pr_diff", "pr_view", "issue_view"}
# In-process bookkeeping: recorded on the Record, never posted as an action.
NOTES = {"candidate", "verdict"}
JOB_NOTES = {"review": NOTES, "mention": set(), "code-review": NOTES}
MAX_BODY = 65536 - len(MARKER) - 1  # GitHub comment limit, less the marker line
TRUNCATED = "\n[truncated]"
Status = Literal["kept", "wrong", "duplicate"]
STATUSES = ("kept", "wrong", "duplicate")
CANDIDATE_ID = re.compile(r"c[1-9]\d*")


@dataclass(frozen=True)
class Target:
    repo: str
    number: int
    is_pr: bool


@dataclass(frozen=True)
class Mention:
    target: Target
    reply_to: int | None  # review-thread root; None means a top-level comment
    context: dict[str, Any]


@dataclass(frozen=True)
class Posted:
    id: int
    review: bool  # review-thread comment (pulls API) rather than issue comment


@dataclass(frozen=True)
class InlineComment:
    path: str
    line: int
    body: str


@dataclass(frozen=True)
class Reply:
    comment_id: int
    body: str


@dataclass(frozen=True)
class Resolve:
    thread_id: str


Action = InlineComment | Reply | Resolve


ACTIONS: dict[str, type[Action]] = {
    "inline_comment": InlineComment,
    "reply": Reply,
    "resolve_thread": Resolve,
}
KINDS = {cls: kind for kind, cls in ACTIONS.items()}


@dataclass
class Candidate:
    path: str
    line: int
    summary: str
    status: Status | None = None
    evidence: str | None = None


@dataclass
class Record:
    """What `run` writes and `apply` reads."""

    job: str
    model: str
    actions: list[Action] = field(default_factory=list)
    summary: str | None = None  # body of the last `comment` call
    footer: str | None = None  # cost line; None without a result message
    error: str | None = None  # result subtype when the run ended in error
    candidates: list[Candidate] = field(default_factory=list)  # id c<n> is index n-1
    head_sha: str | None = None  # PR head the run saw; None for mention
    diff_unavailable: bool = (
        False  # GitHub did not serve the diff, inline lines unchecked
    )


@dataclass(frozen=True)
class Job:
    name: str
    prompt: str
    model: str
    effort: EffortLevel
    append: str | None
    tools: list[SdkMcpTool]
    builtin: list[str]
    settings: list[SettingSource]
    cwd: Path | None  # None: the process cwd
    record: Record
    record_path: Path
    secrets: Mapping[str, str]


@dataclass(frozen=True)
class Thread:
    author: str
    body: str
    repo: str
    number: int


class GitHub(Protocol):
    async def pr_diff(self, pr: int) -> str: ...
    async def compare_diff(self, base: str, head: str) -> str: ...
    async def pr_view(self, pr: int) -> str: ...
    async def issue_view(self, number: int) -> str: ...
    async def head_sha(self, pr: int) -> str: ...
    async def pr_payload(self, pr: int) -> dict: ...
    async def inline_comment(
        self, pr: int, path: str, line: int, body: str, sha: str
    ) -> None: ...
    async def reply(self, pr: int, comment_id: int, body: str) -> Posted: ...
    async def comment(self, number: int, body: str) -> Posted: ...
    async def edit_comment(self, comment: Posted, body: str) -> None: ...
    async def thread(self, thread_id: str) -> Thread: ...
    async def resolve_thread(self, thread_id: str) -> None: ...


Runner = Callable[[Job], AsyncIterator[Message]]


# Pure core


def target(event: dict) -> Target:
    item = event["issue"] if "issue" in event else event["pull_request"]
    return Target(
        event["repository"]["full_name"],
        item["number"],
        "pull_request" in event or "pull_request" in item,
    )


def mention_from_event(name: str, event: dict) -> Mention:
    item = event["issue"] if "issue" in event else event["pull_request"]
    comment = event.get("comment")
    review = event.get("review")
    reply_to = None
    if name == "pull_request_review_comment":
        root = event["comment"]
        reply_to = root.get("in_reply_to_id") or root["id"]
    context = {
        "event_name": name,
        "title": item["title"],
        "body": item["body"],
        "author": (comment or review or item)["user"]["login"],
        "comment_body": comment and comment["body"],
        "path": comment and comment.get("path"),
        "line": comment and comment.get("line"),
        "diff_hunk": comment and comment.get("diff_hunk"),
        "review_body": review and review["body"],
    }
    context = {k: v for k, v in context.items() if v is not None}
    return Mention(target(event), reply_to, context)


def review_prompt(target: Target, head_sha: str, doc: str) -> str:
    header = f"REPO: {target.repo}\nPR NUMBER: {target.number}\nHEAD SHA: {head_sha}"
    return f"{header}\n\n{doc}"


def mention_prompt(m: Mention, doc: str) -> str:
    event = json.dumps(m.context, indent=2)
    return f"REPO: {m.target.repo}\n\n{doc}\n\nEvent:\n\n{event}\n"


def code_review_prompt(target: Target, effort: str) -> str:
    return f"/code-review {effort} --comment {target.number}"


def diff_lines(diff: str) -> dict[str, set[int]]:
    """RIGHT-side line numbers inside a hunk (added or context), per file in a unified diff."""
    result: dict[str, set[int]] = {}
    path: str | None = None
    old = new = line_no = 0  # lines left in the current hunk, next RIGHT line number
    for line in diff.split("\n"):
        if old > 0 or new > 0:
            tag = line[:1]
            if tag == "\\":
                continue
            if tag != "+":
                old -= 1
            if tag != "-":
                new -= 1
                if path:
                    result[path].add(line_no)
                line_no += 1
        elif line.startswith("+++ "):
            name = unquote(line[4:].removesuffix("\t"))
            path = name.removeprefix("b/") if name.startswith("b/") else None
            if path:
                result.setdefault(path, set())
        elif m := HUNK.match(line):
            old = int(m[1] or 1)
            line_no = int(m[2])
            new = int(m[3] or 1)
    return result


def unquote(name: str) -> str:
    """Decode a C-style quoted name as git writes non-ASCII or special paths."""
    if len(name) < 2 or not (name[0] == name[-1] == '"'):
        return name

    def byte(m: re.Match[bytes]) -> bytes:
        return bytes([int(m[1], 8)]) if m[1] else C_ESCAPES[m[2]]

    return ESCAPE.sub(byte, name[1:-1].encode()).decode()


def check_in_diff(lines: Mapping[str, set[int]], path: str, line: int) -> None:
    if path not in lines:
        raise ValueError(f"{path} is not a file changed in the PR diff")
    if line not in lines[path]:
        raise ValueError(
            f"line {line} of {path} is not inside a diff hunk; "
            "use a RIGHT-side line shown in the diff"
        )


def github_error(e: Exception) -> str:
    """One line for a failed call: status, GitHub's message and its error details."""
    if not isinstance(e, RequestFailed):
        return " ".join(str(e).split())[:200]
    resp = e.response
    try:
        body = json.loads(resp.text)
    except ValueError:
        return f"{resp.status_code} {' '.join(resp.text.split())[:200]}"
    details = [
        err if isinstance(err, str) else err.get("message") or err.get("code", "")
        for err in body.get("errors", [])
    ]
    parts = [str(resp.status_code), body.get("message", ""), *details]
    return " ".join(p for p in parts if p)[:300]


def review_comment_error(body: str, pr: Mapping) -> str | None:
    words = (body.splitlines() or [""])[0].split()
    if words != ["/review"]:
        return f"expected a bare /review, got: {' '.join(words)}"
    head = pr["head"]["repo"]
    # Same scope as the PR-open review: the review job may resolve threads.
    if (
        head is None
        or head["full_name"] != pr["base"]["repo"]["full_name"]
        or "dependabot" in pr["user"]["login"]
    ):
        return "/review runs on same-repo, non-Dependabot PRs only; use /code-review"
    return None


def check_body(body: str, secrets: Mapping[str, str]) -> None:
    for name, value in secrets.items():
        if value and value in body:
            raise ValueError(f"body contains {name}, refusing")


def options(job: Job, server: McpSdkServerConfig) -> ClaudeAgentOptions:
    prompt: SystemPromptPreset = {"type": "preset", "preset": "claude_code"}
    if job.append:
        prompt["append"] = job.append
    return ClaudeAgentOptions(
        model=job.model,
        effort=job.effort,
        max_turns=MAX_TURNS,
        tools=job.builtin,
        allowed_tools=job.builtin + [f"mcp__{SERVER}"],
        disallowed_tools=DISALLOWED,
        setting_sources=job.settings,
        cwd=job.cwd,
        system_prompt=prompt,
        mcp_servers={SERVER: server},
        # Only the `gh` server: no .mcp.json from the checkout.
        strict_mcp_config=True,
        permission_mode="dontAsk",
        env=CLI_ENV,
        # Local runs on NixOS: the bundled CLI binary does not execute there.
        cli_path=os.environ.get("CLAUDE_CLI_PATH"),
    )


def select_tools(tools: list[SdkMcpTool], job_name: str) -> list[SdkMcpTool]:
    allowed = JOB_WRITES[job_name] | JOB_NOTES[job_name]
    return [t for t in tools if t.name not in WRITES | NOTES or t.name in allowed]


def is_note(name: str) -> bool:
    """Note tools log their own line from the handler."""
    return name.removeprefix(f"mcp__{SERVER}__") in NOTES


def tool_line(name: str, args: Mapping[str, Any], cwd: Path) -> str:
    parts = []
    for key, value in args.items():
        text = f"<{len(value)} chars>" if key == "body" else str(value)
        text = text.removeprefix(f"{cwd}/").replace("\n", " ")[:100]
        parts.append(f"{key}={text}")
    return f"tool {name.removeprefix(f'mcp__{SERVER}__')} {' '.join(parts)}".rstrip()


def result_lines(result: ResultMessage | None) -> tuple[list[str], int]:
    """Log lines for a run and its exit code."""
    if result is None:
        return ["no result message"], 1
    lines = [f"result {len(result.result)} chars"] if result.result else []
    code = 0
    if result.is_error:
        lines.append(f"result is_error, subtype={result.subtype}")
        code = 1
    denials = json.dumps(result.permission_denials)
    lines.append(
        f"total_cost_usd={result.total_cost_usd} "
        f"duration_s={result.duration_ms / 1000:.0f} permission_denials={denials}"
    )
    return lines, code


def footer(result: ResultMessage, model: str) -> str:
    cost = "n/a" if result.total_cost_usd is None else f"${result.total_cost_usd:.2f}"
    minutes, seconds = divmod(round(result.duration_ms / 1000), 60)
    duration = f"{minutes}m {seconds}s" if minutes else f"{seconds}s"
    return f"<sub>Cost {cost} · {duration} · {model}</sub>"


def one_line(text: str, limit: int = 120) -> str:
    return " ".join(text.split())[:limit]


def add_candidate(rec: Record, path: str, line: int, summary: str) -> str:
    rec.candidates.append(Candidate(path, line, summary))
    cid = f"c{len(rec.candidates)}"
    log.info("candidate %s %s:%d %s", cid, path, line, one_line(summary))
    return cid


def set_verdict(rec: Record, cid: str, status: str, evidence: str) -> str:
    index = candidate_index(rec, cid)
    if index is None:
        raise ValueError(f"unknown candidate id {cid!r}")
    match status:
        case "wrong" if not evidence.strip():
            raise ValueError("wrong needs evidence quoted from the code")
        case "duplicate" if not names_other(rec, index, evidence):
            raise ValueError("duplicate needs evidence naming another candidate id")
        case "kept" | "wrong" | "duplicate":
            c = rec.candidates[index]
            c.status, c.evidence = status, evidence
        case _:
            raise ValueError(
                f"status must be one of {', '.join(STATUSES)}, got {status!r}"
            )
    log.info("verdict %s %s %s", cid, status, one_line(evidence))
    return "recorded"


def names_other(rec: Record, index: int, evidence: str) -> bool:
    ids = (candidate_index(rec, m) for m in re.findall(r"\bc\d+\b", evidence))
    return any(i not in (None, index) for i in ids)


def candidate_index(rec: Record, cid: str) -> int | None:
    if not CANDIDATE_ID.fullmatch(cid):
        return None
    index = int(cid[1:]) - 1
    return index if index < len(rec.candidates) else None


def candidates_section(candidates: list[Candidate]) -> str | None:
    if not candidates:
        return None
    counts = {s: sum(c.status == s for c in candidates) for s in STATUSES}
    unjudged = len(candidates) - sum(counts.values())
    lines = []
    for i, c in enumerate(candidates, 1):
        head = f"- c{i} {c.path}:{c.line} {one_line(c.summary, 200)}"
        if c.status == "wrong":
            lines.append(f"{head}: wrong: {one_line(c.evidence or '', 300)}")
        elif c.status is None:
            lines.append(f"{head}: unjudged")
    title = (
        f"Candidates: {counts['kept']} kept, {counts['wrong']} wrong, "
        f"{counts['duplicate']} duplicate, {unjudged} unjudged"
    )
    return "\n".join([f"<details><summary>{title}</summary>", "", *lines, "</details>"])


def noun(job: str) -> str:
    return "Answer" if job == "mention" else "Review"


def status_body(
    record: Record, pr: int, run_result: str, run_url: str, skipped: list[str]
) -> str:
    """Fits MAX_BODY: drops the candidates section first, then truncates the summary."""
    what = noun(record.job)
    reason = record.error or (run_result if run_result != "success" else None)

    def render(summary: str | None, section: str | None) -> str:
        parts = []
        if record.head_sha:
            parts.append(f"### Review of #{pr} at {record.head_sha[:7]}")
        if summary:
            parts.append(summary)
        if record.diff_unavailable:
            parts.append("Diff unavailable; inline lines unchecked.")
        if skipped:
            parts.append("**Not applied**\n" + "\n".join(skipped))
        if reason:
            parts.append(f"{what} incomplete: {reason}. [run]({run_url})")
        elif not summary:
            parts.append(f"{what} finished without a summary")
        if section:
            parts.append(section)
        if record.footer:
            parts.append(record.footer)
        return "\n\n".join(parts)

    section = candidates_section(record.candidates)
    body = render(record.summary, section)
    if len(body) <= MAX_BODY:
        return body
    if section:
        body = render(record.summary, None)
    if len(body) > MAX_BODY and record.summary:
        keep = max(len(record.summary) - (len(body) - MAX_BODY) - len(TRUNCATED), 0)
        body = render(record.summary[:keep] + TRUNCATED, None)
    return body


def to_json(record: Record) -> dict[str, Any]:
    data = asdict(record)
    data["actions"] = [{"kind": KINDS[type(a)], **asdict(a)} for a in record.actions]
    return data


def parse_action(data: Any) -> Action:
    fields = dict(data)
    kind = fields.pop("kind")
    if kind not in ACTIONS:
        raise ValueError(f"unknown action kind {kind!r}")
    return ACTIONS[kind](**fields)


def parse_record(data: Any, job: str) -> Record:
    fields = dict(data)
    actions = [parse_action(a) for a in fields.pop("actions")]
    candidates = [Candidate(**c) for c in fields.pop("candidates")]
    record = Record(**fields, actions=actions, candidates=candidates)
    if record.job != job:
        raise ValueError(f"record is for job {record.job!r}, expected {job!r}")
    return record


def check_action(action: Action) -> None:
    match action:
        case InlineComment(line=line) if line <= 0:
            raise ValueError(f"line must be positive, got {line}")
        case Reply(comment_id=comment_id) if comment_id <= 0:
            raise ValueError(f"comment_id must be positive, got {comment_id}")
        case Resolve(thread_id=thread_id) if not THREAD_ID.fullmatch(thread_id):
            raise ValueError(f"invalid thread_id {thread_id!r}")


def label(action: Action) -> str:
    match action:
        case InlineComment(path=path, line=line):
            return f"inline_comment {path}:{line}"
        case Reply(comment_id=comment_id):
            return f"reply {comment_id}"
        case Resolve(thread_id=thread_id):
            return f"resolve_thread {thread_id}"


def parse_thread(data: dict[str, Any]) -> Thread:
    node = data["node"]
    if not node or "pullRequest" not in node:
        raise ValueError("id is not a review thread")
    pr = node["pullRequest"]
    root = node["comments"]["nodes"][0]
    return Thread(
        root["author"]["login"],
        root["body"],
        pr["repository"]["nameWithOwner"],
        pr["number"],
    )


def login(user: Any) -> str:
    return "ghost" if user is None else user.login  # deleted accounts


def view_data(item: Any) -> dict[str, Any]:
    return {
        "title": item.title,
        "body": item.body,
        "author": login(item.user),
        "state": item.state,
    }


def view_json(item: Any) -> str:
    return json.dumps(view_data(item))


# Tools


def make_tools(
    gh: GitHub,
    target: Target,
    secrets: Mapping[str, str],
    rec: Record,
    lines: Mapping[str, set[int]] | None,
    save: Callable[[], None],
    read_diff: Callable[[], Awaitable[str]],
) -> list[SdkMcpTool]:
    """Reads act on `target`; the model never names a PR, issue or repo.

    Builds every tool; `select_tools` drops the writes a job may not use. Writes and notes
    update `rec`, call `save` and post nothing. With `lines` (see `diff_lines`), inline
    comments outside the diff are refused. `pr_diff` returns `read_diff()`. Handlers raise
    ValueError on refused calls, which the SDK reports to the model as an error result.
    """
    n = target.number

    def define(
        name: str,
        desc: str,
        schema: dict[str, type],
        run: Callable[[Any], Awaitable[str]],
    ):
        async def handler(args: dict[str, Any]) -> dict[str, Any]:
            for value in args.values():
                if isinstance(value, str):
                    check_body(value, secrets)
            out = await run(args)
            if name not in READS:
                save()
            return {"content": [{"type": "text", "text": out}]}

        return tool(name, desc, schema)(handler)

    async def add(action: Action) -> str:
        check_action(action)
        if lines is not None and isinstance(action, InlineComment):
            check_in_diff(lines, action.path, action.line)
        rec.actions.append(action)
        return "recorded"

    async def comment(a: dict[str, Any]) -> str:
        rec.summary = a["body"]
        return "recorded"

    async def note_candidate(a: dict[str, Any]) -> str:
        return add_candidate(rec, a["path"], a["line"], a["summary"])

    async def note_verdict(a: dict[str, Any]) -> str:
        return set_verdict(rec, a["id"], a["status"], a["evidence"])

    if target.is_pr:
        reads = [
            define("pr_diff", "Diff of the PR.", {}, lambda a: read_diff()),
            define(
                "pr_view",
                "Title, body and author of the PR.",
                {},
                lambda a: gh.pr_view(n),
            ),
        ]
    else:
        reads = [
            define(
                "issue_view",
                "Title, body and author of the issue.",
                {},
                lambda a: gh.issue_view(n),
            )
        ]
    return reads + [
        define(
            "inline_comment",
            "Review comment on a line of the PR head, RIGHT side.",
            {"path": str, "line": int, "body": str},
            lambda a: add(InlineComment(a["path"], a["line"], a["body"])),
        ),
        define(
            "reply",
            "Reply in a review thread; comment_id is the thread root comment.",
            {"comment_id": int, "body": str},
            lambda a: add(Reply(a["comment_id"], a["body"])),
        ),
        define(
            "comment",
            "Final summary comment; the last call is posted.",
            {"body": str},
            comment,
        ),
        define(
            "resolve_thread",
            "Resolve a review thread started by claude or by this tool.",
            {"thread_id": str},
            lambda a: add(Resolve(a["thread_id"])),
        ),
        define(
            "candidate",
            "Note a possible defect the moment it is noticed; returns its id.",
            {"path": str, "line": int, "summary": str},
            note_candidate,
        ),
        define(
            "verdict",
            "Judge a candidate: status kept, wrong (evidence: quoted code it misreads) or "
            "duplicate (evidence: the candidate id it duplicates, e.g. c3).",
            {"id": str, "status": str, "evidence": str},
            note_verdict,
        ),
    ]


# GitHub implementations


def marked(body: str) -> str:
    return f"{MARKER}\n{body}"


class GitHubKit:
    def __init__(self, repo: str, token: str | None):
        """`token` None: unauthenticated, public reads only."""
        self.owner, self.name = repo.split("/")
        auth = UnauthAuthStrategy() if token is None else TokenAuthStrategy(token)
        self.gh = GitHubClient(auth)

    async def pr_diff(self, pr: int) -> str:
        resp = await self.gh.arequest(
            "GET",
            f"/repos/{self.owner}/{self.name}/pulls/{pr}",
            headers={"Accept": "application/vnd.github.diff"},
        )
        return resp.text

    async def compare_diff(self, base: str, head: str) -> str:
        resp = await self.gh.arequest(
            "GET",
            f"/repos/{self.owner}/{self.name}/compare/{base}...{head}",
            headers={"Accept": "application/vnd.github.diff"},
        )
        return resp.text

    async def pr_view(self, pr: int) -> str:
        resp = await self.gh.rest.pulls.async_get(
            owner=self.owner, repo=self.name, pull_number=pr
        )
        return view_json(resp.parsed_data)

    async def issue_view(self, number: int) -> str:
        resp = await self.gh.rest.issues.async_get(
            owner=self.owner, repo=self.name, issue_number=number
        )
        return view_json(resp.parsed_data)

    async def head_sha(self, pr: int) -> str:
        resp = await self.gh.rest.pulls.async_get(
            owner=self.owner, repo=self.name, pull_number=pr
        )
        return resp.parsed_data.head.sha

    async def pr_payload(self, pr: int) -> dict:
        resp = await self.gh.arequest(
            "GET", f"/repos/{self.owner}/{self.name}/pulls/{pr}"
        )
        return resp.json()

    async def inline_comment(
        self, pr: int, path: str, line: int, body: str, sha: str
    ) -> None:
        await self.gh.rest.pulls.async_create_review_comment(
            owner=self.owner,
            repo=self.name,
            pull_number=pr,
            body=marked(body),
            commit_id=sha,
            path=path,
            line=line,
            side="RIGHT",
        )

    async def reply(self, pr: int, comment_id: int, body: str) -> Posted:
        resp = await self.gh.rest.pulls.async_create_reply_for_review_comment(
            owner=self.owner,
            repo=self.name,
            pull_number=pr,
            comment_id=comment_id,
            body=marked(body),
        )
        return Posted(resp.parsed_data.id, review=True)

    async def comment(self, number: int, body: str) -> Posted:
        resp = await self.gh.rest.issues.async_create_comment(
            owner=self.owner, repo=self.name, issue_number=number, body=marked(body)
        )
        return Posted(resp.parsed_data.id, review=False)

    async def edit_comment(self, comment: Posted, body: str) -> None:
        if comment.review:
            await self.gh.rest.pulls.async_update_review_comment(
                owner=self.owner,
                repo=self.name,
                comment_id=comment.id,
                body=marked(body),
            )
        else:
            await self.gh.rest.issues.async_update_comment(
                owner=self.owner,
                repo=self.name,
                comment_id=comment.id,
                body=marked(body),
            )

    async def thread(self, thread_id: str) -> Thread:
        data = await self.gh.async_graphql(THREAD_QUERY, {"id": thread_id})
        return parse_thread(data)

    async def resolve_thread(self, thread_id: str) -> None:
        await self.gh.async_graphql(RESOLVE_MUTATION, {"id": thread_id})


THREAD_QUERY = """
query($id: ID!) {
  node(id: $id) {
    ... on PullRequestReviewThread {
      pullRequest { number repository { nameWithOwner } }
      comments(first: 1) { nodes { author { login } body } }
    }
  }
}"""

RESOLVE_MUTATION = """
mutation($id: ID!) {
  resolveReviewThread(input: {threadId: $id}) { thread { isResolved } }
}"""


# Start and apply (write token, no Claude)


async def start(
    gh: GitHub, job: str, event_name: str, event: dict, run_url: str
) -> Posted:
    n = target(event).number
    if job == "review":
        if "pull_request" not in event:
            err = review_comment_error(event["comment"]["body"], await gh.pr_payload(n))
            if err:
                raise SystemExit(f"::error::{err}")
        # A /review comment event has no pull_request: head not known yet.
        sha = (
            f" {event['pull_request']['head']['sha'][:7]}"
            if "pull_request" in event
            else ""
        )
        text = f"Reviewing{sha}… [run]({run_url})"
    else:
        text = f"Working on it… [run]({run_url})"
    root = mention_from_event(event_name, event).reply_to if job == "mention" else None
    if root:
        return await gh.reply(n, root, text)
    return await gh.comment(n, text)


async def print_start(
    gh: GitHub, job: str, event_name: str, event: dict, run_url: str
) -> int:
    """Prints only the status comment id: the workflow captures stdout."""
    print((await start(gh, job, event_name, event, run_url)).id)
    return 0


async def check_thread(gh: GitHub, tgt: Target, thread_id: str) -> None:
    thread = await gh.thread(thread_id)
    if (thread.repo, thread.number) != (tgt.repo, tgt.number):
        raise ValueError(
            f"thread {thread_id} belongs to {thread.repo}#{thread.number}, "
            f"not {tgt.repo}#{tgt.number}"
        )
    ours = thread.author == "claude" or (
        thread.author == "github-actions" and thread.body.startswith(MARKER)
    )
    if not ours:
        raise ValueError(
            f"thread {thread_id} was started by {thread.author!r}, not resolving"
        )


async def post(
    gh: GitHub, job: str, tgt: Target, sha: str | None, action: Action
) -> None:
    kind = KINDS[type(action)]
    if kind not in JOB_WRITES[job]:
        raise ValueError(f"{kind} not allowed in {job}")
    check_action(action)
    match action:
        case InlineComment(path, line, body):
            assert sha is not None
            await gh.inline_comment(tgt.number, path, line, body, sha)
        case Reply(comment_id, body):
            await gh.reply(tgt.number, comment_id, body)
        case Resolve(thread_id):
            await check_thread(gh, tgt, thread_id)
            await gh.resolve_thread(thread_id)


async def apply(
    gh: GitHub,
    job: str,
    tgt: Target,
    raw: str | None,
    status: Posted,
    run_url: str,
    run_result: str,
) -> int:
    """Post each action independently, then edit the status. Returns the exit code.

    Inline comments bind to the head sha `run` recorded; without one it is fetched once.
    """
    what = noun(job)
    if raw is None:
        reason = "no actions file" if run_result == "success" else run_result
        await gh.edit_comment(status, f"{what} failed: {reason}. [run]({run_url})")
        return 1
    try:
        record = parse_record(json.loads(raw), job)
    except (ValueError, TypeError, KeyError) as e:
        log.error("invalid record: %r", e)
        await gh.edit_comment(
            status, f"{what} failed: invalid actions. [run]({run_url})"
        )
        raise
    # A cancelled run was superseded; its partial actions would duplicate the next run's.
    actions = [] if run_result == "cancelled" else record.actions
    head_sha = record.head_sha
    if head_sha is None and any(isinstance(a, InlineComment) for a in actions):
        head_sha = await gh.head_sha(tgt.number)
    skipped = []
    for action in actions:
        try:
            await post(gh, job, tgt, head_sha, action)
        except Exception as e:  # noqa: BLE001 best effort: any failure skips this action only
            reason = github_error(e)
            log.error("skipped %s: %s", label(action), reason)
            skipped.append(f"- {label(action)}: {reason}")
    body = status_body(record, tgt.number, run_result, run_url, skipped)
    try:
        await gh.edit_comment(status, body)
    except RequestFailed as e:
        log.error("status edit rejected: %s", github_error(e))
        await gh.edit_comment(
            status, f"{what} summary could not be posted. [run]({run_url})"
        )
    return 0 if run_result == "success" else 1


# Run (read token, Claude)


async def fetch_diff(gh: GitHub, base: str, head: str) -> str | None:
    """Diff of `base...head`; None when GitHub cannot serve it, auth failures raise."""
    try:
        return await gh.compare_diff(base, head)
    except RequestFailed as e:
        if e.response.status_code in (401, 403):
            raise
        log.warning("diff refused, inline lines unchecked: %s", github_error(e))
        return None


def served(diff: str | None) -> Callable[[], Awaitable[str]]:
    """`pr_diff` for a prefetched diff: the same text the inline lines are checked against."""

    async def read() -> str:
        if diff is None:
            raise ValueError("GitHub did not serve the diff")
        return diff

    return read


def write_record(path: Path, record: Record) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    part = path.with_name(path.name + ".part")
    part.write_text(json.dumps(to_json(record)))
    os.replace(part, path)  # a killed run never leaves a truncated record


async def run(job: Job, runner: Runner) -> int:
    result = None
    cwd = (job.cwd or Path.cwd()).absolute()
    rec = job.record
    try:
        async for message in runner(job):
            if isinstance(message, AssistantMessage):
                for block in message.content:
                    if isinstance(block, TextBlock):
                        log.info("text %s", one_line(block.text, 300))
                    elif isinstance(block, ToolUseBlock) and not is_note(block.name):
                        log.info(tool_line(block.name, block.input, cwd))
            elif isinstance(message, ResultMessage):
                result = message
    finally:
        if result:
            rec.footer = footer(result, job.model)
            rec.error = result.subtype if result.is_error else None
            if job.name == "mention" and rec.summary is None and result.result:
                answer_fallback(rec, result.result, job.secrets)
        write_record(job.record_path, rec)
    lines, code = result_lines(result)
    for line in lines:
        log.info(line)
    return code


def answer_fallback(rec: Record, text: str, secrets: Mapping[str, str]) -> None:
    """A mention answered in text, without `comment`."""
    try:
        check_body(text, secrets)
    except ValueError as e:
        rec.error = f"answer text: {e}"
        return
    rec.summary = text


def sdk_runner(job: Job) -> AsyncIterator[Message]:
    server = create_sdk_mcp_server(SERVER, tools=job.tools)
    return query(prompt=job.prompt, options=options(job, server))


def load_secrets(env: Mapping[str, str]) -> dict[str, str]:
    secrets = {"GH_TOKEN": env["GH_TOKEN"]}
    if key := env.get("ANTHROPIC_API_KEY"):  # unset: CLI uses the local login
        secrets["ANTHROPIC_API_KEY"] = key
    return secrets


def work_tmp(args: argparse.Namespace, tmp: Path) -> Path:
    """Where Claude reads its files: `tmp` under its cwd. The record stays in `tmp`."""
    return tmp if args.workdir is None else args.workdir / "tmp"


def build_job(
    args: argparse.Namespace,
    gh: GitHub,
    secrets: Mapping[str, str],
    event_name: str,
    event: dict,
    head_sha: str | None,
    diff: str | None,
    tmp: Path,
) -> Job:
    """`head_sha` and `diff` are None for mention; `diff` also when GitHub refused it."""
    append = None
    builtin = BUILTIN
    settings: list[SettingSource] = ["user"]  # PR head checkout: no project settings
    record = Record(
        args.job,
        args.model,
        head_sha=head_sha,
        diff_unavailable=args.job != "mention" and diff is None,
    )
    if args.job == "review":
        assert head_sha is not None
        tgt = target(event)
        backport = (work_tmp(args, tmp) / "is-backport").exists()
        focus = "review-backport.md" if backport else "review-standard.md"
        doc = (
            (CLAUDE_DIR / focus).read_text()
            + "\n\n"
            + (CLAUDE_DIR / "review.md").read_text()
        )
        prompt = review_prompt(tgt, head_sha, doc)
    elif args.job == "mention":
        mention = mention_from_event(event_name, event)
        tgt = mention.target
        prompt = mention_prompt(mention, (CLAUDE_DIR / "mention.md").read_text())
        settings = ["user", "project"]  # default-branch checkout
    else:
        tgt = target(event)
        prompt = code_review_prompt(tgt, args.effort)
        append = (CLAUDE_DIR / "code-review.md").read_text()
        builtin = BUILTIN + ["Agent"]
    path = tmp / "actions.json"
    lines = None if diff is None else diff_lines(diff)
    read_diff = (
        (lambda: gh.pr_diff(tgt.number)) if args.job == "mention" else served(diff)
    )
    tools = make_tools(
        gh, tgt, secrets, record, lines, lambda: write_record(path, record), read_diff
    )
    return Job(
        args.job,
        prompt,
        args.model,
        args.effort,
        append,
        select_tools(tools, args.job),
        builtin,
        settings,
        args.workdir,
        record,
        path,
        secrets,
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ("start", "run", "apply"):
        p = sub.add_parser(command)
        p.add_argument("job", choices=JOB_WRITES)
        if command == "run":
            p.add_argument("--model", default="opus")
            p.add_argument("--effort", default="high", choices=get_args(EffortLevel))
            p.add_argument("--workdir", type=Path, help="Claude's cwd")
            p.add_argument(
                "--head-sha", help="PR head checked out; review: else the event's"
            )
            p.add_argument("--base-sha", help="PR base; review: else the event's")
        if command == "apply":
            p.add_argument("--status-id", type=int, required=True)
            p.add_argument("--run-result", required=True)
            p.add_argument("--record", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.command == "run":
        given = (args.head_sha is not None, args.base_sha is not None)
        if (
            given[0] != given[1]
            or (args.job == "code-review" and not given[0])
            or (args.job == "mention" and given[0])
        ):
            parser.error(
                "--head-sha and --base-sha: both or neither; required for code-review,"
                " not allowed for mention"
            )
    return args


def main() -> int:
    logging.basicConfig(
        level=logging.INFO,
        format="%(levelname)s %(name)s: %(message)s",
        stream=sys.stderr,
    )
    args = parse_args(sys.argv[1:])
    gh = GitHubKit(os.environ["GITHUB_REPOSITORY"], os.environ["GH_TOKEN"])
    return asyncio.run(amain(args, gh, os.environ, sdk_runner, TMP))


async def amain(
    args: argparse.Namespace,
    gh: GitHub,
    env: Mapping[str, str],
    runner: Runner,
    tmp: Path,
) -> int:
    event = json.loads(Path(env["GITHUB_EVENT_PATH"]).read_text())
    repo = env["GITHUB_REPOSITORY"]
    run_url = f"{env['GITHUB_SERVER_URL']}/{repo}/actions/runs/{env['GITHUB_RUN_ID']}"
    name = env["GITHUB_EVENT_NAME"]
    if args.command == "start":
        return await print_start(gh, args.job, name, event, run_url)
    if args.command == "apply":
        root = (
            mention_from_event(name, event).reply_to if args.job == "mention" else None
        )
        raw = args.record.read_text() if args.record.exists() else None
        status = Posted(args.status_id, review=root is not None)
        return await apply(
            gh, args.job, target(event), raw, status, run_url, args.run_result
        )
    head_sha = diff = None
    if args.job != "mention":
        head_sha, base_sha = args.head_sha, args.base_sha
        if head_sha is None:
            pr = event["pull_request"]
            head_sha, base_sha = pr["head"]["sha"], pr["base"]["sha"]
        diff = await fetch_diff(gh, base_sha, head_sha)
    if args.job == "review" and diff is not None:
        claude_tmp = work_tmp(args, tmp)
        claude_tmp.mkdir(parents=True, exist_ok=True)
        (claude_tmp / "pr.diff").write_text(diff)
    secrets = load_secrets(env)
    job = build_job(args, gh, secrets, name, event, head_sha, diff, tmp)
    return await run(job, runner)


if __name__ == "__main__":
    sys.exit(main())
