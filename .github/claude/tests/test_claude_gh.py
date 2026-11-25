"""Tests for `claude_gh`: tool recording, apply validation, status texts and exit codes, with fake GitHub and agent.

No network or SDK calls. Run with:

    just claude::test
"""

import asyncio
import json
import logging
from collections.abc import AsyncIterator
from dataclasses import dataclass, field
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import pytest
from claude_agent_sdk import (
    AssistantMessage,
    Message,
    ResultMessage,
    SdkMcpTool,
    TextBlock,
    ToolUseBlock,
    create_sdk_mcp_server,
)
from githubkit.exception import RequestFailed

import claude_gh as cg

PR = cg.Target("o/r", 7, True)
ISSUE = cg.Target("o/r", 9, False)
SECRETS = {"ANTHROPIC_API_KEY": "sk-secret", "GH_TOKEN": "ghs_secret"}
RUN = "http://run"
STATUS = cg.Posted(99, False)
SHA = "abcdef123456"
HEADER = "### Review of #7 at abcdef1"
BASE = "0123base"
SERVER = create_sdk_mcp_server("gh", tools=[])
SAME_REPO_PR = {
    "head": {"repo": {"full_name": "o/r"}},
    "base": {"repo": {"full_name": "o/r"}},
    "user": {"login": "alice"},
}


@dataclass
class FakeGitHub:
    threads: dict[str, dict] = field(default_factory=dict)
    fail_on: set[str] = field(default_factory=set)
    posted: list[tuple[str, tuple]] = field(default_factory=list)
    resolved: list[str] = field(default_factory=list)
    edits: list[tuple[cg.Posted, str]] = field(default_factory=list)
    head_fetches: int = 0
    diff: str | RequestFailed = "diff"
    live_diffs: int = 0
    compares: list[tuple[str, str]] = field(default_factory=list)
    edit_max: int | None = None  # edits longer than this raise
    pr: dict = field(default_factory=lambda: dict(SAME_REPO_PR))

    async def pr_diff(self, pr: int) -> str:
        self.live_diffs += 1
        return "live diff"

    async def compare_diff(self, base: str, head: str) -> str:
        self.compares.append((base, head))
        if isinstance(self.diff, RequestFailed):
            raise self.diff
        return self.diff

    async def pr_view(self, pr: int) -> str:
        return f"pr {pr}"

    async def issue_view(self, number: int) -> str:
        return f"issue {number}"

    async def head_sha(self, pr: int) -> str:
        self.head_fetches += 1
        return "fetched"

    async def pr_payload(self, pr: int) -> dict:
        return self.pr

    async def inline_comment(
        self, pr: int, path: str, line: int, body: str, sha: str
    ) -> None:
        if body in self.fail_on:
            raise RuntimeError("422 line outside the diff")
        self.posted.append(("inline", (pr, path, line, body, sha)))

    async def reply(self, pr: int, comment_id: int, body: str) -> cg.Posted:
        if body in self.fail_on:
            raise RuntimeError("422 line outside the diff")
        self.posted.append(("reply", (pr, comment_id, body)))
        return cg.Posted(len(self.posted), True)

    async def comment(self, number: int, body: str) -> cg.Posted:
        self.posted.append(("comment", (number, body)))
        return cg.Posted(len(self.posted), False)

    async def edit_comment(self, comment: cg.Posted, body: str) -> None:
        if self.edit_max is not None and len(body) > self.edit_max:
            raise failed(422, "body is too long")
        self.edits.append((comment, body))

    async def thread(self, thread_id: str) -> cg.Thread:
        return cg.parse_thread(self.threads[thread_id])

    async def resolve_thread(self, thread_id: str) -> None:
        self.resolved.append(thread_id)


def make_result(**kw: Any) -> ResultMessage:
    fields: dict[str, Any] = {
        "subtype": "success",
        "duration_ms": 2000,
        "duration_api_ms": 1000,
        "is_error": False,
        "num_turns": 3,
        "session_id": "s",
        "total_cost_usd": 0.5,
        "result": "done",
        "permission_denials": [],
    }
    return ResultMessage(**{**fields, **kw})


@dataclass
class FakeRunner:
    """Calls each (tool, args) through the job's handlers, then yields `result`."""

    calls: list[tuple[str, dict]]
    result: ResultMessage | None
    outputs: list[dict] = field(default_factory=list)
    jobs: list[Any] = field(default_factory=list)

    async def __call__(self, job: Any) -> AsyncIterator[Message]:
        self.jobs.append(job)
        by_name = {t.name: t for t in job.tools}
        for name, args in self.calls:
            block = ToolUseBlock(id=name, name=f"mcp__gh__{name}", input=args)
            yield AssistantMessage(content=[block], model="m")
            self.outputs.append(await by_name[name].handler(args))
        if self.result:
            yield self.result


def tools_by_name(tools: list[SdkMcpTool]) -> dict[str, SdkMcpTool]:
    return {t.name: t for t in tools}


def make(gh: FakeGitHub, target: cg.Target = PR, name: str = "review"):
    rec = cg.Record(name, "opus")
    live = lambda: gh.pr_diff(target.number)
    return cg.make_tools(gh, target, SECRETS, rec, None, lambda: None, live), rec


def call(tools: list[SdkMcpTool], name: str, args: dict) -> dict:
    return asyncio.run(tools_by_name(tools)[name].handler(args))  # ty: ignore[invalid-argument-type]


def text(out: dict) -> str:
    return out["content"][0]["text"]


def thread(
    author: str = "claude", repo: str = "o/r", number: int = 7, body: str = "b"
) -> dict:
    return {
        "node": {
            "pullRequest": {"number": number, "repository": {"nameWithOwner": repo}},
            "comments": {"nodes": [{"author": {"login": author}, "body": body}]},
        }
    }


def job(tools: list[SdkMcpTool] | None = None, **kw: Any) -> Any:
    fields: dict[str, Any] = {
        "name": "review",
        "prompt": "p",
        "model": "opus",
        "effort": "high",
        "append": None,
        "tools": tools or [],
        "builtin": ["Read", "Grep", "Glob"],
        "settings": ["user"],
        "cwd": None,
        "record": cg.Record("review", "opus"),
        "record_path": Path("unused"),
        "secrets": SECRETS,
    }
    return cg.Job(**{**fields, **kw})


def test_options_no_bash():
    t, _ = make(FakeGitHub())
    o = cg.options(job(t), SERVER)
    assert o.cwd is None
    assert o.mcp_servers == {"gh": SERVER}
    assert o.strict_mcp_config is True
    assert o.tools == ["Read", "Grep", "Glob"]
    assert "Bash" not in o.allowed_tools
    assert "mcp__gh" in o.allowed_tools
    assert o.disallowed_tools == [
        "Read(//proc/**)",
        "Grep(//proc/**)",
        "Glob(//proc/**)",
    ]
    assert o.setting_sources == ["user"]
    assert o.permission_mode == "dontAsk"
    assert o.max_turns == 120
    assert o.env == {"GH_TOKEN": "", "CLAUDE_CODE_DISABLE_BACKGROUND_TASKS": "1"}


def test_options_builtin_and_settings_per_job():
    o = cg.options(job(builtin=["Read", "Agent"], settings=["user", "project"]), SERVER)
    assert o.tools == ["Read", "Agent"]
    assert o.allowed_tools == ["Read", "Agent", "mcp__gh"]
    assert o.setting_sources == ["user", "project"]
    assert cg.options(job(cwd=Path("pr")), SERVER).cwd == Path("pr")


def test_options_model_effort_and_append():
    o = cg.options(job(model="sonnet", effort="max", append="extra"), SERVER)
    assert (o.model, o.effort) == ("sonnet", "max")
    assert o.system_prompt == {
        "type": "preset",
        "preset": "claude_code",
        "append": "extra",
    }


def test_write_tools_record_actions_and_post_nothing():
    gh = FakeGitHub()
    t, rec = make(gh)
    calls = [
        ("inline_comment", {"path": "a.rs", "line": 3, "body": "i"}),
        ("reply", {"comment_id": 5, "body": "r"}),
        ("resolve_thread", {"thread_id": "T_1"}),
        ("comment", {"body": "s"}),
    ]
    for name, args in calls:
        assert text(call(t, name, args)) == "recorded"
    assert gh.posted == [] and gh.resolved == []
    assert cg.to_json(rec) == {
        "job": "review",
        "model": "opus",
        "summary": "s",
        "actions": [
            {"kind": "inline_comment", "path": "a.rs", "line": 3, "body": "i"},
            {"kind": "reply", "comment_id": 5, "body": "r"},
            {"kind": "resolve_thread", "thread_id": "T_1"},
        ],
        "footer": None,
        "error": None,
        "candidates": [],
        "head_sha": None,
        "diff_unavailable": False,
    }


def test_summary_last_comment_wins():
    t, rec = make(FakeGitHub())
    call(t, "comment", {"body": "first"})
    call(t, "comment", {"body": "last"})
    assert rec.summary == "last" and rec.actions == []


def test_tools_have_no_target_arg():
    t, _ = make(FakeGitHub())
    for tl in t:
        assert not {"pr", "number", "repo"} & set(tl.input_schema)


def test_pr_tools_vs_issue_tools():
    pr = tools_by_name(make(FakeGitHub())[0])
    issue = tools_by_name(make(FakeGitHub(), ISSUE)[0])
    assert {"pr_diff", "pr_view"} <= set(pr) and "issue_view" not in pr
    assert "issue_view" in issue and "pr_diff" not in issue


NOTES = {"candidate", "verdict"}


@pytest.mark.parametrize(
    "name,writes,notes",
    [
        ("mention", {"comment"}, set()),
        ("code-review", {"inline_comment", "reply", "comment"}, NOTES),
        ("review", {"inline_comment", "reply", "comment", "resolve_thread"}, NOTES),
    ],
)
def test_job_tools_pinned(name, writes, notes):
    assert cg.JOB_WRITES[name] == writes
    names = set(tools_by_name(cg.select_tools(make(FakeGitHub())[0], name)))
    assert names == {"pr_diff", "pr_view"} | writes | notes


def test_candidate_ids_sequential_and_logged(caplog):
    caplog.set_level(logging.INFO, logger="claude-gh")
    t, rec = make(FakeGitHub())
    ids = [
        text(call(t, "candidate", {"path": "a.rs", "line": i, "summary": f"s{i}"}))
        for i in (1, 2, 3)
    ]
    assert ids == ["c1", "c2", "c3"]
    call(t, "candidate", {"path": "b.rs", "line": 9, "summary": "multi\nline " * 30})
    assert [c.line for c in rec.candidates] == [1, 2, 3, 9]
    assert rec.actions == [] and rec.summary is None
    last = caplog.records[-1].getMessage()
    assert last.startswith("candidate c4 b.rs:9 multi line multi")
    assert "\n" not in last and len(last) < 160


def test_verdict_sets_status_and_evidence(caplog):
    caplog.set_level(logging.INFO, logger="claude-gh")
    t, rec = make(FakeGitHub())
    for line in (1, 2, 3):
        call(t, "candidate", {"path": "a.rs", "line": line, "summary": "s"})
    out = call(t, "verdict", {"id": "c2", "status": "wrong", "evidence": "`x?`"})
    assert text(out) == "recorded"
    call(t, "verdict", {"id": "c3", "status": "duplicate", "evidence": "same as c1"})
    call(t, "verdict", {"id": "c1", "status": "kept", "evidence": ""})
    assert [(c.status, c.evidence) for c in rec.candidates] == [
        ("kept", ""),
        ("wrong", "`x?`"),
        ("duplicate", "same as c1"),
    ]
    assert caplog.records[-2].getMessage() == "verdict c3 duplicate same as c1"


@pytest.mark.parametrize(
    "args,match",
    [
        ({"id": "c3", "status": "kept"}, "unknown candidate id"),
        ({"id": "c0", "status": "kept"}, "unknown candidate id"),
        ({"id": "1", "status": "kept"}, "unknown candidate id"),
        ({"id": "c1", "status": "maybe"}, "kept, wrong, duplicate"),
        (
            {"id": "c1", "status": "dropped", "evidence": "`x`"},
            "kept, wrong, duplicate",
        ),
        ({"id": "c1", "status": "wrong"}, "needs evidence"),
        ({"id": "c1", "status": "wrong", "evidence": " \n"}, "needs evidence"),
        ({"id": "c1", "status": "duplicate"}, "another candidate id"),
        ({"id": "c1", "status": "duplicate", "evidence": "same as c1"}, "another"),
        ({"id": "c1", "status": "duplicate", "evidence": "same as c3"}, "another"),
        ({"id": "c1", "status": "duplicate", "evidence": "same as c0"}, "another"),
        ({"id": "c1", "status": "duplicate", "evidence": "see abc2"}, "another"),
    ],
)
def test_verdict_errors(args, match):
    t, rec = make(FakeGitHub())
    call(t, "candidate", {"path": "a.rs", "line": 1, "summary": "s"})
    call(t, "candidate", {"path": "a.rs", "line": 2, "summary": "s"})
    with pytest.raises(ValueError, match=match):
        call(t, "verdict", {"evidence": "", **args})
    assert rec.candidates == [
        cg.Candidate("a.rs", 1, "s"),
        cg.Candidate("a.rs", 2, "s"),
    ]


@pytest.mark.parametrize(
    "name,args",
    [
        ("candidate", {"path": "a", "line": 1, "summary": "leak sk-secret"}),
        ("verdict", {"id": "c1", "status": "wrong", "evidence": "leak sk-secret"}),
    ],
)
def test_secret_in_notes_refused(name, args):
    t, rec = make(FakeGitHub())
    rec.candidates.append(cg.Candidate("a", 1, "s"))
    with pytest.raises(ValueError, match="refusing"):
        call(t, name, args)
    assert rec.candidates == [cg.Candidate("a", 1, "s")]


@pytest.mark.parametrize(
    "name,args",
    [
        ("candidate", {"path": "sk-secret", "line": 1, "summary": "s"}),
        ("inline_comment", {"path": "sk-secret", "line": 1, "body": "b"}),
        ("resolve_thread", {"thread_id": "sk-secret"}),
    ],
)
def test_secret_in_any_string_arg_refused(name, args):
    t, rec = make(FakeGitHub())
    with pytest.raises(ValueError, match="refusing"):
        call(t, name, args)
    assert rec.actions == [] and rec.candidates == []


def test_reads_return_text():
    t, _ = make(FakeGitHub())
    assert text(call(t, "pr_diff", {})) == "live diff"
    assert text(call(t, "pr_view", {})) == "pr 7"


@pytest.mark.parametrize("secret", ["sk-secret", "ghs_secret"])
@pytest.mark.parametrize(
    "name,args",
    [
        ("inline_comment", {"path": "a", "line": 1}),
        ("reply", {"comment_id": 1}),
        ("comment", {}),
    ],
)
def test_secret_body_refused(name, args, secret):
    t, rec = make(FakeGitHub())
    with pytest.raises(ValueError, match="refusing"):
        call(t, name, {**args, "body": f"leak {secret} here"})
    assert rec.actions == [] and rec.summary is None


def test_load_secrets_api_key_optional():
    assert cg.load_secrets({"GH_TOKEN": "ghs_x"}) == {"GH_TOKEN": "ghs_x"}
    env = {"GH_TOKEN": "ghs_x", "ANTHROPIC_API_KEY": "sk-x"}
    assert cg.load_secrets(env) == env


def test_load_secrets_requires_gh_token():
    with pytest.raises(KeyError):
        cg.load_secrets({})


def test_empty_secret_never_blocks():
    cg.check_body("anything", {"EMPTY": ""})
    t, _ = make(FakeGitHub())
    assert "is_error" not in call(t, "comment", {"body": "anything"})


def test_resolve_bad_thread_id_refused():
    t, rec = make(FakeGitHub())
    with pytest.raises(ValueError, match="invalid thread_id"):
        call(t, "resolve_thread", {"thread_id": 'x") { y } #'})
    assert rec.actions == []


@pytest.mark.parametrize("line", [0, -1])
def test_inline_nonpositive_line_refused(line):
    t, rec = make(FakeGitHub())
    with pytest.raises(ValueError, match="positive"):
        call(t, "inline_comment", {"path": "a", "line": line, "body": "x"})
    assert rec.actions == []


def comment_event(**comment: Any) -> dict:
    return {
        "repository": {"full_name": "o/r"},
        "pull_request": {
            "number": 7,
            "title": "T",
            "body": "B",
            "user": {"login": "bob"},
        },
        "comment": {
            "id": 11,
            "body": "@claude hi",
            "user": {"login": "amy"},
            **comment,
        },
    }


ISSUE_COMMENT_EVENT = {
    "repository": {"full_name": "o/r"},
    "issue": {
        "number": 9,
        "title": "T",
        "body": None,
        "user": {"login": "bob"},
        "pull_request": {},
    },
    "comment": {"id": 1, "body": "@claude", "user": {"login": "amy"}},
}


def test_mention_review_comment_replies_to_thread_root():
    m = cg.mention_from_event(
        "pull_request_review_comment", comment_event(in_reply_to_id=4)
    )
    assert m.reply_to == 4 and m.target == PR
    m = cg.mention_from_event("pull_request_review_comment", comment_event())
    assert m.reply_to == 11


def test_mention_issue_comment_is_top_level():
    m = cg.mention_from_event("issue_comment", ISSUE_COMMENT_EVENT)
    assert m.reply_to is None and m.target == cg.Target("o/r", 9, True)
    assert m.context["author"] == "amy" and "body" not in m.context
    assert "number" not in m.context


def test_mention_issue_event_is_not_pr():
    event = {
        "repository": {"full_name": "o/r"},
        "issue": {
            "number": 9,
            "title": "@claude",
            "body": "b",
            "user": {"login": "bob"},
        },
    }
    m = cg.mention_from_event("issues", event)
    assert m.target == ISSUE and m.context["author"] == "bob"


def test_mention_review_event_uses_review_author():
    event = {
        "repository": {"full_name": "o/r"},
        "pull_request": {
            "number": 7,
            "title": "T",
            "body": "B",
            "user": {"login": "bob"},
        },
        "review": {"body": "@claude look", "user": {"login": "amy"}},
    }
    m = cg.mention_from_event("pull_request_review", event)
    assert m.context["author"] == "amy" and m.context["review_body"] == "@claude look"
    assert m.reply_to is None


def test_mention_prompt_embeds_event_as_plain_context():
    m = cg.mention_from_event("pull_request_review_comment", comment_event())
    prompt = cg.mention_prompt(m, "DOC")
    assert prompt.startswith("REPO: o/r\n\nDOC")
    assert '"comment_body": "@claude hi"' in prompt
    assert "untrusted" not in prompt


def test_code_review_prompt_is_one_slash_line():
    assert cg.code_review_prompt(PR, "xhigh") == "/code-review xhigh --comment 7"
    event = {
        "repository": {"full_name": "o/r"},
        "issue": {"number": 7, "pull_request": {}},
    }
    assert cg.target(event) == PR


def test_review_prompt_header_and_doc():
    p = cg.review_prompt(PR, "abc123", "DOC")
    assert p == "REPO: o/r\nPR NUMBER: 7\nHEAD SHA: abc123\n\nDOC"


def test_target_from_pull_request_event():
    event = {"repository": {"full_name": "o/r"}, "pull_request": {"number": 7}}
    assert cg.target(event) == PR


def test_tool_line_has_no_bodies_or_newlines():
    line = cg.tool_line(
        "mcp__gh__inline_comment",
        {"path": "a.rs", "line": 3, "body": "secret\nbody"},
        Path("/w"),
    )
    assert line == "tool inline_comment path=a.rs line=3 body=<11 chars>"
    assert "\n" not in cg.tool_line("Grep", {"pattern": "a\nb" * 100}, Path("/w"))


def test_tool_line_paths_relative_to_cwd():
    cwd = Path("/home/runner/work/espresso-network/espresso-network/pr")
    line = cg.tool_line(
        "Read", {"file_path": f"{cwd}/tmp/pr-review-comments.json"}, cwd
    )
    assert line == "tool Read file_path=tmp/pr-review-comments.json"


def run_job(tmp_path: Path, calls: list, result: ResultMessage | None, runner=None):
    t, rec = make(FakeGitHub())
    path = tmp_path / "tmp" / "actions.json"
    j = job(t, record=rec, record_path=path)
    runner = runner or FakeRunner(calls, result)
    return j, path, runner


def test_run_ok_logs_final_line_and_writes_record(caplog, tmp_path):
    caplog.set_level(logging.INFO, logger="claude-gh")
    j, path, runner = run_job(
        tmp_path,
        [("comment", {"body": "hi"})],
        make_result(permission_denials=[{"t": 1}]),
    )
    assert asyncio.run(cg.run(j, runner)) == 0
    messages = [r.getMessage() for r in caplog.records]
    assert messages == [
        "tool comment body=<2 chars>",
        "result 4 chars",
        'total_cost_usd=0.5 duration_s=2 permission_denials=[{"t": 1}]',
    ]
    assert caplog.records[-1].levelno == logging.INFO
    assert json.loads(path.read_text()) == {
        "job": "review",
        "model": "opus",
        "summary": "hi",
        "actions": [],
        "footer": cg.footer(make_result(), "opus"),
        "error": None,
        "candidates": [],
        "head_sha": None,
        "diff_unavailable": False,
    }


def test_run_logs_text_as_one_truncated_line(caplog, tmp_path):
    caplog.set_level(logging.INFO, logger="claude-gh")

    async def runner(job: Any) -> AsyncIterator[Message]:
        yield AssistantMessage(content=[TextBlock("a\n\nb " + "x" * 400)], model="m")
        yield make_result()

    j, _, _ = run_job(tmp_path, [], None)
    asyncio.run(cg.run(j, runner))
    line = caplog.records[0].getMessage()
    assert line == "text a b " + "x" * 296


def test_run_error_records_subtype_and_partial_actions(tmp_path):
    j, path, runner = run_job(
        tmp_path,
        [("reply", {"comment_id": 1, "body": "r"})],
        make_result(is_error=True, subtype="error_max_turns"),
    )
    assert asyncio.run(cg.run(j, runner)) == 1
    record = cg.parse_record(json.loads(path.read_text()), "review")
    assert record.actions == [cg.Reply(1, "r")]
    assert record.error == "error_max_turns"


def test_run_missing_result_fails_and_records(tmp_path):
    j, path, runner = run_job(tmp_path, [("comment", {"body": "s"})], None)
    assert asyncio.run(cg.run(j, runner)) == 1
    record = cg.parse_record(json.loads(path.read_text()), "review")
    assert record.footer is None and record.summary == "s"


def test_run_raising_runner_still_writes_record(tmp_path):
    class Boom:
        async def __call__(self, job: Any) -> AsyncIterator[Message]:
            raise RuntimeError("boom")
            yield

    j, path, _ = run_job(tmp_path, [], None)
    j.record.summary = "partial"
    with pytest.raises(RuntimeError, match="boom"):
        asyncio.run(cg.run(j, Boom()))
    assert cg.parse_record(json.loads(path.read_text()), "review").summary == "partial"


@pytest.mark.parametrize(
    "name,summary,expected",
    [("mention", None, "done"), ("mention", "s", "s"), ("review", None, None)],
)
def test_mention_answer_falls_back_to_result_text(tmp_path, name, summary, expected):
    j, path, runner = run_job(tmp_path, [], make_result())
    j = job(
        name=name, record=cg.Record(name, "opus", summary=summary), record_path=path
    )
    asyncio.run(cg.run(j, runner))
    assert cg.parse_record(json.loads(path.read_text()), name).summary == expected


def test_mention_answer_fallback_with_secret_is_refused(tmp_path):
    j, path, runner = run_job(tmp_path, [], make_result(result="tok ghs_secret"))
    j = job(name="mention", record=cg.Record("mention", "opus"), record_path=path)
    asyncio.run(cg.run(j, runner))
    rec = cg.parse_record(json.loads(path.read_text()), "mention")
    assert rec.summary is None
    assert rec.error == "answer text: body contains GH_TOKEN, refusing"


def test_write_record_replaces_whole_file(tmp_path):
    path = tmp_path / "actions.json"
    cg.write_record(path, cg.Record("review", "opus", summary="a"))
    cg.write_record(path, cg.Record("review", "opus", summary="b"))
    assert cg.parse_record(json.loads(path.read_text()), "review").summary == "b"
    assert [f.name for f in tmp_path.iterdir()] == ["actions.json"]


def test_record_written_after_each_recording_call(tmp_path):
    """A killed run keeps what was recorded: the file is current before the next call."""
    path = tmp_path / "actions.json"
    rec = cg.Record("review", "opus")
    snapshots: list[dict] = []

    def save():
        cg.write_record(path, rec)
        snapshots.append(json.loads(path.read_text()))

    t = cg.make_tools(FakeGitHub(), PR, SECRETS, rec, None, save, cg.served("d"))
    call(t, "pr_diff", {})
    assert snapshots == []
    call(t, "reply", {"comment_id": 1, "body": "r"})
    call(t, "candidate", {"path": "a", "line": 1, "summary": "s"})
    call(t, "comment", {"body": "sum"})
    with pytest.raises(ValueError):
        call(t, "reply", {"comment_id": 0, "body": "r"})
    assert [len(s["actions"]) for s in snapshots] == [1, 1, 1]
    assert [len(s["candidates"]) for s in snapshots] == [0, 1, 1]
    assert snapshots[-1]["summary"] == "sum"


def test_run_is_error_fails():
    lines, code = cg.result_lines(make_result(is_error=True, subtype="error_max_turns"))
    assert code == 1
    assert "subtype=error_max_turns" in "\n".join(lines)
    assert lines[-1].startswith("total_cost_usd=")


def test_cost_none_prints():
    lines, code = cg.result_lines(make_result(total_cost_usd=None, result=None))
    assert code == 0
    assert lines == ["total_cost_usd=None duration_s=2 permission_denials=[]"]


def test_parse_args():
    args = cg.parse_args(["run", *CODE_REVIEW_ARGV, "--model", "m", "--effort", "max"])
    assert (args.job, args.model, args.effort) == ("code-review", "m", "max")
    args = cg.parse_args(["run", "review"])
    assert (args.model, args.effort, args.workdir) == ("opus", "high", None)
    assert cg.parse_args(["run", "mention", "--workdir", "pr"]).workdir == Path("pr")
    with pytest.raises(SystemExit):
        cg.parse_args(["run", "review", "--effort", "extreme"])
    args = cg.parse_args(
        [
            "apply",
            "mention",
            "--status-id",
            "5",
            "--run-result",
            "failure",
            "--record",
            "r",
        ]
    )
    assert args.status_id == 5 and args.record == Path("r")


def test_parse_args_shas_both_or_neither_required_for_code_review():
    argv = ["run", "code-review", "--head-sha", "h", "--base-sha", "b"]
    args = cg.parse_args(argv)
    assert (args.head_sha, args.base_sha) == ("h", "b")
    for bad in (
        ["run", "code-review"],
        ["run", "code-review", "--head-sha", "h"],
        ["run", "review", "--head-sha", "h"],
        ["run", "mention", "--head-sha", "h", "--base-sha", "b"],
    ):
        with pytest.raises(SystemExit):
            cg.parse_args(bad)
    args = cg.parse_args(["run", "review", "--head-sha", "h", "--base-sha", "b"])
    assert (args.head_sha, args.base_sha) == ("h", "b")


def test_parse_thread():
    assert cg.parse_thread(thread()) == cg.Thread("claude", "b", "o/r", 7)


@pytest.mark.parametrize("data", [{"node": {}}, {"node": None}, {"node": {"id": "x"}}])
def test_parse_thread_rejects_wrong_node(data):
    with pytest.raises(ValueError, match="not a review thread"):
        cg.parse_thread(data)


def test_fetch_diff_compares_shas():
    gh = FakeGitHub()
    assert asyncio.run(cg.fetch_diff(gh, BASE, SHA)) == "diff"
    assert gh.compares == [(BASE, SHA)] and gh.live_diffs == 0


@pytest.mark.parametrize("status", [404, 406, 422, 500])
def test_fetch_diff_unavailable_is_none(status):
    gh = FakeGitHub(diff=failed(status, "x"))
    assert asyncio.run(cg.fetch_diff(gh, BASE, SHA)) is None


@pytest.mark.parametrize("status", [401, 403])
def test_fetch_diff_auth_errors_raise(status):
    with pytest.raises(RequestFailed):
        asyncio.run(cg.fetch_diff(FakeGitHub(diff=failed(status, "x")), BASE, SHA))


def test_view_json_is_compact_and_has_no_comments():
    user = SimpleNamespace(login="amy")
    item = SimpleNamespace(title="T", body="B", user=user, state="open")
    out = cg.view_json(item)
    assert "\n" not in out
    assert json.loads(out) == {
        "title": "T",
        "body": "B",
        "author": "amy",
        "state": "open",
    }


def test_view_json_deleted_user_is_ghost():
    item = SimpleNamespace(title="T", body="B", user=None, state="open")
    assert json.loads(cg.view_json(item))["author"] == "ghost"


# Records and apply

FOOT = cg.footer(make_result(), "opus")


def record(
    actions: list | None = None,
    summary: str | None = "sum",
    job_name: str = "review",
    footer: str | None = FOOT,
    error: str | None = None,
    head_sha: str | None = SHA,
) -> cg.Record:
    return cg.Record(
        job_name, "opus", actions or [], summary, footer, error, head_sha=head_sha
    )


def do_apply(
    gh: FakeGitHub,
    rec: cg.Record | str | None,
    job_name: str = "review",
    run_result: str = "success",
    tgt: cg.Target = PR,
    status: cg.Posted = STATUS,
) -> int:
    raw = json.dumps(cg.to_json(rec)) if isinstance(rec, cg.Record) else rec
    return asyncio.run(cg.apply(gh, job_name, tgt, raw, status, RUN, run_result))


ALL = [
    cg.InlineComment("a.rs", 3, "i"),
    cg.Reply(5, "r"),
    cg.Resolve("T_1"),
]


CANDIDATES = [
    cg.Candidate("a.rs", 3, "kept one", "kept", ""),
    cg.Candidate("b.rs", 5, "wrong one", "wrong", "`if x > 0`\nguards it"),
    cg.Candidate("c.rs", 7, "open one"),
    cg.Candidate("d.rs", 9, "dup one", "duplicate", "c1"),
]


def test_record_json_roundtrip():
    rec = record(ALL, error="error_max_turns")
    assert cg.parse_record(cg.to_json(rec), "review") == rec
    rec.candidates = list(CANDIDATES)
    assert cg.parse_record(json.loads(json.dumps(cg.to_json(rec))), "review") == rec
    rec = record(summary=None, footer=None, head_sha=None)
    rec.diff_unavailable = True
    assert cg.parse_record(cg.to_json(rec), "review") == rec


def test_apply_posts_all_in_order_and_edits_status():
    gh = FakeGitHub(threads={"T_1": thread()})
    assert do_apply(gh, record(ALL)) == 0
    assert gh.posted == [
        ("inline", (7, "a.rs", 3, "i", SHA)),
        ("reply", (7, 5, "r")),
    ]
    assert gh.resolved == ["T_1"]
    assert gh.edits == [(STATUS, f"{HEADER}\n\nsum\n\n{FOOT}")]
    assert gh.head_fetches == 0


def test_apply_fetches_head_once_without_record_sha():
    gh = FakeGitHub()
    actions = [cg.InlineComment("a.rs", 1, "x"), cg.InlineComment("a.rs", 2, "y")]
    assert do_apply(gh, record(actions, head_sha=None)) == 0
    assert gh.head_fetches == 1
    assert [p[1][4] for p in gh.posted] == ["fetched", "fetched"]
    assert gh.edits[-1][1] == f"sum\n\n{FOOT}"
    do_apply(gh, record([cg.Reply(1, "r")], head_sha=None))
    assert gh.head_fetches == 1


def test_apply_notes_diff_unavailable():
    gh = FakeGitHub()
    rec = record()
    rec.diff_unavailable = True
    do_apply(gh, rec)
    body = f"{HEADER}\n\nsum\n\nDiff unavailable; inline lines unchecked.\n\n{FOOT}"
    assert gh.edits == [(STATUS, body)]


@pytest.mark.parametrize(
    "author,body",
    [("claude", "b"), ("github-actions", f"{cg.MARKER}\nb")],
)
def test_resolve_own_thread(author, body):
    gh = FakeGitHub(threads={"T_1": thread(author, body=body)})
    assert do_apply(gh, record([cg.Resolve("T_1")])) == 0
    assert gh.resolved == ["T_1"]


def test_apply_appends_candidates_section():
    gh = FakeGitHub()
    rec = record()
    rec.candidates = list(CANDIDATES)
    assert do_apply(gh, rec) == 0
    assert gh.posted == []
    section = (
        "<details><summary>"
        "Candidates: 1 kept, 1 wrong, 1 duplicate, 1 unjudged</summary>\n\n"
        "- c2 b.rs:5 wrong one: wrong: `if x > 0` guards it\n"
        "- c3 c.rs:7 open one: unjudged\n"
        "</details>"
    )
    assert gh.edits == [(STATUS, f"{HEADER}\n\nsum\n\n{section}\n\n{FOOT}")]


def test_apply_without_candidates_has_no_section():
    gh = FakeGitHub()
    do_apply(gh, record())
    assert "<details>" not in gh.edits[-1][1]


def test_run_logs_no_tool_line_for_notes(caplog, tmp_path):
    caplog.set_level(logging.INFO, logger="claude-gh")
    calls = [("candidate", {"path": "a.rs", "line": 1, "summary": "s"})]
    j, path, runner = run_job(tmp_path, calls, make_result())
    asyncio.run(cg.run(j, runner))
    assert caplog.records[0].getMessage() == "candidate c1 a.rs:1 s"
    assert not any(r.getMessage().startswith("tool ") for r in caplog.records)
    assert json.loads(path.read_text())["candidates"] == [
        {"path": "a.rs", "line": 1, "summary": "s", "status": None, "evidence": None}
    ]


def test_apply_without_summary_says_so():
    gh = FakeGitHub()
    do_apply(gh, record(summary=None))
    body = f"{HEADER}\n\nReview finished without a summary\n\n{FOOT}"
    assert gh.edits == [(STATUS, body)]


@pytest.mark.parametrize(
    "bad",
    [
        "{",
        "[]",
        json.dumps({"job": "review"}),
        json.dumps({**cg.to_json(record()), "extra": 1}),
        json.dumps({**cg.to_json(record()), "actions": [{"kind": "comment"}]}),
        json.dumps({**cg.to_json(record()), "actions": [{"kind": "reply", "x": 1}]}),
        json.dumps({**cg.to_json(record()), "actions": [{"comment_id": 1}]}),
    ],
)
def test_unusable_record_refused_whole(bad):
    gh = FakeGitHub()
    with pytest.raises((ValueError, TypeError, KeyError)):
        do_apply(gh, bad)
    assert gh.posted == []
    assert gh.edits == [(STATUS, "Review failed: invalid actions. [run](http://run)")]


def test_job_mismatch_refused_whole():
    gh = FakeGitHub()
    with pytest.raises(ValueError, match="expected 'review'"):
        do_apply(gh, record(job_name="code-review"), job_name="review")
    assert gh.posted == [] and "failed" in gh.edits[-1][1]


@pytest.mark.parametrize(
    "name,action,line",
    [
        ("code-review", cg.Resolve("T_1"), "resolve_thread T_1: resolve_thread not"),
        ("mention", cg.Reply(1, "x"), "reply 1: reply not allowed in mention"),
    ],
)
def test_kind_not_allowed_is_skipped(name, action, line):
    gh = FakeGitHub(threads={"T_1": thread()})
    assert do_apply(gh, record([action], job_name=name), job_name=name) == 0
    assert gh.posted == [] and gh.resolved == []
    assert f"**Not applied**\n- {line}" in gh.edits[-1][1]


@pytest.mark.parametrize(
    "other,match",
    [
        (thread("alice"), "alice"),
        (thread("github-actions", body="Test log"), "github-actions"),
        (thread("github-actions", body=f"x\n{cg.MARKER}"), "github-actions"),
        (thread(number=8), "belongs to"),
        (thread(repo="o/x"), "belongs to"),
        ({"node": {}}, "not a review thread"),
    ],
)
def test_bad_thread_is_skipped(other, match):
    gh = FakeGitHub(threads={"T_1": other})
    assert do_apply(gh, record([cg.Resolve("T_1")])) == 0
    assert gh.resolved == []
    assert match in gh.edits[-1][1] and "**Not applied**" in gh.edits[-1][1]


@pytest.mark.parametrize(
    "action",
    [
        cg.InlineComment("a.rs", 0, "x"),
        cg.Reply(0, "x"),
        cg.Resolve('x") { y } #'),
    ],
)
def test_invalid_action_is_skipped(action):
    gh = FakeGitHub()
    assert do_apply(gh, record([action])) == 0
    assert gh.posted == [] and gh.resolved == []


def test_github_error_skips_only_that_action():
    gh = FakeGitHub(fail_on={"bad"})
    actions = [
        cg.Reply(1, "first"),
        cg.InlineComment("a.rs", 9, "bad"),
        cg.Reply(2, "third"),
    ]
    assert do_apply(gh, record(actions)) == 0
    assert gh.posted == [("reply", (7, 1, "first")), ("reply", (7, 2, "third"))]
    assert gh.edits == [
        (
            STATUS,
            f"{HEADER}\n\nsum\n\n**Not applied**\n"
            + "- inline_comment a.rs:9: 422 line outside the diff"
            + f"\n\n{FOOT}",
        )
    ]


@pytest.mark.parametrize(
    "outcome,posted", [("failure", [("reply", (7, 5, "r"))]), ("cancelled", [])]
)
def test_failed_run_posts_partial_record_unless_cancelled(outcome, posted):
    gh = FakeGitHub()
    rec = record([cg.Reply(5, "r")], footer=None)
    assert do_apply(gh, rec, run_result=outcome) == 1
    assert gh.posted == posted
    assert gh.edits == [
        (STATUS, f"{HEADER}\n\nsum\n\nReview incomplete: {outcome}. [run](http://run)")
    ]


def test_error_subtype_is_the_incomplete_reason():
    gh = FakeGitHub()
    rec = record([cg.Reply(5, "r")], error="error_max_turns")
    assert do_apply(gh, rec, run_result="failure") == 1
    assert gh.posted == [("reply", (7, 5, "r"))]
    assert gh.edits == [
        (
            STATUS,
            (
                f"{HEADER}\n\nsum\n\n"
                f"Review incomplete: error_max_turns. [run](http://run)\n\n{FOOT}"
            ),
        )
    ]


def test_status_body_fits_comment_limit():
    big = "x" * (cg.MAX_BODY + 100)
    rec = record(summary="y" * 1000)
    rec.candidates = [cg.Candidate("a.rs", 1, "z" * 200)] * 20
    assert "Candidates" in cg.status_body(rec, 7, "success", RUN, [])
    rec.candidates *= 20
    body = cg.status_body(rec, 7, "success", RUN, [])
    assert len(body) <= cg.MAX_BODY and "y" * 1000 in body and "Candidates" not in body
    rec.summary = big
    body = cg.status_body(rec, 7, "success", RUN, [])
    assert len(body) <= cg.MAX_BODY and body.startswith(HEADER) and FOOT in body


def test_apply_falls_back_when_edit_is_rejected():
    gh = FakeGitHub(edit_max=200)
    assert do_apply(gh, record(summary="s" * 500)) == 0
    assert gh.edits == [
        (STATUS, "Review summary could not be posted. [run](http://run)")
    ]


def test_missing_record_marks_status_failed():
    gh = FakeGitHub()
    assert do_apply(gh, None, run_result="cancelled") == 1
    assert gh.edits == [(STATUS, "Review failed: cancelled. [run](http://run)")]
    assert do_apply(gh, None) == 1
    assert gh.edits[-1][1] == "Review failed: no actions file. [run](http://run)"
    assert gh.posted == []


def test_mention_status_uses_answer_wording():
    gh = FakeGitHub()
    status = cg.Posted(5, True)
    rec = record(job_name="mention", summary="answer", head_sha=None)
    assert do_apply(gh, rec, "mention", tgt=ISSUE, status=status) == 0
    assert gh.edits == [(status, f"answer\n\n{FOOT}")]
    do_apply(gh, None, "mention")
    assert gh.edits[-1][1].startswith("Answer failed: no actions file")
    empty = record(None, None, "mention", head_sha=None)
    do_apply(gh, empty, "mention", run_result="failure")
    assert "Answer incomplete: failure" in gh.edits[-1][1]
    do_apply(gh, empty, "mention")
    assert "Answer finished without a summary" in gh.edits[-1][1]


def test_start_in_review_thread_replies_to_root():
    gh = FakeGitHub()
    posted = asyncio.run(
        cg.start(
            gh,
            "mention",
            "pull_request_review_comment",
            comment_event(in_reply_to_id=4),
            RUN,
        )
    )
    assert gh.posted == [("reply", (7, 4, f"Working on it… [run]({RUN})"))]
    assert posted.review is True


@pytest.mark.parametrize(
    "name,event", [("issue_comment", ISSUE_COMMENT_EVENT), ("issues", comment_event())]
)
def test_start_top_level_comments(name, event):
    gh = FakeGitHub()
    posted = asyncio.run(cg.start(gh, "mention", name, event, RUN))
    assert [k for k, _ in gh.posted] == ["comment"]
    assert posted.review is False


def test_start_review_names_head_and_code_review_is_generic():
    event = {
        "repository": {"full_name": "o/r"},
        "pull_request": {"number": 7, "head": {"sha": "abcdef123456"}},
    }
    gh = FakeGitHub()
    asyncio.run(cg.start(gh, "review", "pull_request", event, RUN))
    asyncio.run(cg.start(gh, "code-review", "pull_request", event, RUN))
    assert gh.posted == [
        ("comment", (7, "Reviewing abcdef1… [run](http://run)")),
        ("comment", (7, "Working on it… [run](http://run)")),
    ]


def test_start_review_from_comment_has_no_head():
    gh = FakeGitHub()
    asyncio.run(cg.start(gh, "review", "issue_comment", REVIEW_COMMENT_EVENT, RUN))
    assert gh.posted == [("comment", (7, "Reviewing… [run](http://run)"))]


def test_start_rejected_review_comment_posts_nothing():
    gh = FakeGitHub(pr={**SAME_REPO_PR, "head": {"repo": None}})
    with pytest.raises(SystemExit, match="same-repo"):
        asyncio.run(cg.start(gh, "review", "issue_comment", REVIEW_COMMENT_EVENT, RUN))
    assert gh.posted == []


@pytest.mark.parametrize(
    "body,pr,error",
    [
        ("/review", {}, None),
        ("/review\r\n", {}, None),
        ("/review\nmore text", {}, None),
        ("  /review  ", {}, None),
        ("/review foo", {}, "bare"),
        ("/Review", {}, "bare"),
        ("/reviewed", {}, "bare"),
        ("", {}, "bare"),
        ("/review", {"head": {"repo": {"full_name": "x/r"}}}, "same-repo"),
        ("/review", {"head": {"repo": None}}, "same-repo"),
        ("/review", {"user": {"login": "dependabot[bot]"}}, "same-repo"),
    ],
)
def test_review_comment_error(body, pr, error):
    msg = cg.review_comment_error(body, {**SAME_REPO_PR, **pr})
    assert msg is None if error is None else error in str(msg)


def test_start_prints_only_the_comment_id(capsys):
    event = ISSUE_COMMENT_EVENT
    gh = FakeGitHub()
    assert asyncio.run(cg.print_start(gh, "mention", "issue_comment", event, RUN)) == 0
    assert capsys.readouterr().out == "1\n"


def test_footer_formats_cost_and_duration():
    r = make_result(total_cost_usd=0.589, duration_ms=133_000)
    assert cg.footer(r, "opus") == "<sub>Cost $0.59 · 2m 13s · opus</sub>"
    r = make_result(total_cost_usd=None, duration_ms=45_000)
    assert cg.footer(r, "opus") == "<sub>Cost n/a · 45s · opus</sub>"


DIFF = r"""diff --git a/old.rs b/new.rs
similarity index 90%
rename from old.rs
rename to new.rs
--- a/old.rs
+++ b/new.rs
@@ -1,3 +1,4 @@
 keep
-gone
+--- not a header
+++ not a header
 keep2
@@ -10 +11,2 @@
-x
+y
+z
\ No newline at end of file
diff --git a/fresh.rs b/fresh.rs
new file mode 100644
--- /dev/null
+++ b/fresh.rs
@@ -0,0 +1,2 @@
+a
+b
diff --git a/dead.rs b/dead.rs
deleted file mode 100644
--- a/dead.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-a
-b
diff --git a/pure.rs b/moved.rs
similarity index 100%
rename from pure.rs
rename to moved.rs
"""


def test_diff_lines():
    assert cg.diff_lines(DIFF) == {
        "new.rs": {1, 2, 3, 4, 11, 12},
        "fresh.rs": {1, 2},
    }


def test_diff_lines_path_with_space_has_no_trailing_tab():
    diff = "--- a/foo bar\t\n+++ b/foo bar\t\n@@ -1 +1 @@\n-a\n+b\n"
    assert cg.diff_lines(diff) == {"foo bar": {1}}


def test_diff_lines_unquotes_c_style_names():
    diff = '--- "a/\\303\\244.md"\n+++ "b/\\303\\244 \\"x\\".md"\n@@ -1 +1 @@\n-a\n+b\n'
    assert cg.diff_lines(diff) == {'\u00e4 "x".md': {1}}


def test_diff_lines_empty():
    assert cg.diff_lines("") == {}


def test_inline_comment_outside_diff_is_refused():
    lines = cg.diff_lines(DIFF)
    rec = cg.Record("review", "opus")
    tools = cg.make_tools(
        FakeGitHub(), PR, SECRETS, rec, lines, lambda: None, cg.served(DIFF)
    )
    with pytest.raises(ValueError, match="not a file"):
        call(tools, "inline_comment", {"path": "dead.rs", "line": 1, "body": "x"})
    with pytest.raises(ValueError, match="not inside a diff hunk"):
        call(tools, "inline_comment", {"path": "new.rs", "line": 5, "body": "x"})
    call(tools, "inline_comment", {"path": "new.rs", "line": 12, "body": "x"})


def failed(status: int, body: str) -> RequestFailed:
    response = SimpleNamespace(
        raw_request=None, raw_response=None, status_code=status, text=body
    )
    return RequestFailed(response)  # ty: ignore[invalid-argument-type]


def test_github_error_shows_message_and_details():
    body = json.dumps(
        {
            "message": "Unprocessable Entity",
            "errors": [
                {"field": "line", "message": "line must be part of the diff"},
                "oops",
            ],
        }
    )
    assert (
        cg.github_error(failed(422, body))
        == "422 Unprocessable Entity line must be part of the diff oops"
    )


def test_github_error_non_json_and_other_exceptions():
    assert cg.github_error(failed(502, "bad\n gateway")) == "502 bad gateway"
    assert cg.github_error(RuntimeError("a\nb")) == "a b"


# Wiring: build_job and amain

REVIEW_EVENT = {
    "repository": {"full_name": "o/r"},
    "pull_request": {"number": 7, "head": {"sha": SHA}, "base": {"sha": BASE}},
}
CODE_REVIEW_EVENT = {
    "repository": {"full_name": "o/r"},
    "issue": {"number": 7, "pull_request": {}},
}
REVIEW_COMMENT_EVENT = {**CODE_REVIEW_EVENT, "comment": {"body": "/review"}}
WRITES_AND_NOTES = {"inline_comment", "reply", "comment", "resolve_thread"} | NOTES


def doc(name: str) -> str:
    return (cg.CLAUDE_DIR / name).read_text()


CODE_REVIEW_ARGV = ["code-review", "--head-sha", SHA, "--base-sha", BASE]


def build(
    tmp_path: Path,
    argv: list[str],
    event: dict,
    sha: str | None,
    diff: str | None,
    gh: FakeGitHub | None = None,
) -> cg.Job:
    args = cg.parse_args(["run", *argv])
    name = "pull_request_review_comment"
    gh = gh or FakeGitHub()
    return cg.build_job(args, gh, SECRETS, name, event, sha, diff, tmp_path)


def test_build_review_job(tmp_path):
    j = build(tmp_path, ["review"], REVIEW_EVENT, SHA, DIFF)
    assert j.prompt == cg.review_prompt(
        PR, SHA, doc("review-standard.md") + "\n\n" + doc("review.md")
    )
    assert j.append is None and j.cwd is None
    assert (j.builtin, j.settings) == (["Read", "Grep", "Glob"], ["user"])
    assert set(tools_by_name(j.tools)) == {"pr_diff", "pr_view"} | WRITES_AND_NOTES
    assert (j.record.head_sha, j.record.diff_unavailable) == (SHA, False)
    assert j.record_path == tmp_path / "actions.json"


def test_build_review_job_backport(tmp_path):
    (tmp_path / "is-backport").write_text("41")
    j = build(tmp_path, ["review"], REVIEW_EVENT, SHA, DIFF)
    body = doc("review-backport.md") + "\n\n" + doc("review.md")
    assert j.prompt == cg.review_prompt(PR, SHA, body)
    assert (j.append, j.builtin) == (None, ["Read", "Grep", "Glob"])


def test_build_review_job_backport_reads_workdir_tmp(tmp_path):
    work = tmp_path / "pr"
    (work / "tmp").mkdir(parents=True)
    (work / "tmp" / "is-backport").write_text("41")
    argv = ["review", "--workdir", str(work)]
    j = build(tmp_path, argv, REVIEW_EVENT, SHA, DIFF)
    body = doc("review-backport.md") + "\n\n" + doc("review.md")
    assert j.prompt == cg.review_prompt(PR, SHA, body)
    assert j.record_path == tmp_path / "actions.json"


def test_build_code_review_job(tmp_path):
    argv = [*CODE_REVIEW_ARGV, "--effort", "max", "--workdir", "pr"]
    j = build(tmp_path, argv, CODE_REVIEW_EVENT, SHA, DIFF)
    assert j.prompt == "/code-review max --comment 7"
    assert j.append == doc("code-review.md")
    assert (j.builtin, j.settings) == (["Read", "Grep", "Glob", "Agent"], ["user"])
    assert j.cwd == Path("pr")
    assert set(tools_by_name(j.tools)) == {
        "pr_diff",
        "pr_view",
    } | WRITES_AND_NOTES - {"resolve_thread"}
    assert j.record.head_sha == SHA


def test_build_mention_job(tmp_path):
    j = build(tmp_path, ["mention"], comment_event(), None, None)
    m = cg.mention_from_event("pull_request_review_comment", comment_event())
    assert j.prompt == cg.mention_prompt(m, doc("mention.md"))
    assert (j.append, j.settings) == (None, ["user", "project"])
    assert set(tools_by_name(j.tools)) == {"pr_diff", "pr_view", "comment"}
    assert (j.record.head_sha, j.record.diff_unavailable) == (None, False)


def test_build_job_validates_lines_and_saves(tmp_path):
    j = build(tmp_path, ["review"], REVIEW_EVENT, SHA, DIFF)
    with pytest.raises(ValueError, match="not inside a diff hunk"):
        call(j.tools, "inline_comment", {"path": "new.rs", "line": 5, "body": "x"})
    call(j.tools, "inline_comment", {"path": "new.rs", "line": 12, "body": "x"})
    saved = cg.parse_record(json.loads(j.record_path.read_text()), "review")
    assert saved.actions == [cg.InlineComment("new.rs", 12, "x")]


def test_build_job_without_diff_skips_validation(tmp_path):
    j = build(tmp_path, CODE_REVIEW_ARGV, CODE_REVIEW_EVENT, SHA, None)
    call(j.tools, "inline_comment", {"path": "any.rs", "line": 5, "body": "x"})
    assert j.record.diff_unavailable is True
    with pytest.raises(ValueError, match="did not serve"):
        call(j.tools, "pr_diff", {})


@pytest.mark.parametrize("argv", [["review"], CODE_REVIEW_ARGV])
def test_pr_diff_serves_prefetched_diff(tmp_path, argv):
    gh = FakeGitHub()
    j = build(tmp_path, argv, REVIEW_EVENT, SHA, DIFF, gh)
    assert text(call(j.tools, "pr_diff", {})) == DIFF
    assert gh.live_diffs == 0


def test_mention_pr_diff_is_live(tmp_path):
    gh = FakeGitHub()
    j = build(tmp_path, ["mention"], comment_event(), None, None, gh)
    assert text(call(j.tools, "pr_diff", {})) == "live diff"
    assert gh.live_diffs == 1


def env_for(tmp_path: Path, name: str, event: dict) -> dict[str, str]:
    path = tmp_path / "event.json"
    path.write_text(json.dumps(event))
    return {
        "GITHUB_EVENT_PATH": str(path),
        "GITHUB_REPOSITORY": "o/r",
        "GITHUB_SERVER_URL": "https://gh",
        "GITHUB_RUN_ID": "5",
        "GITHUB_EVENT_NAME": name,
        "GH_TOKEN": "ghs_secret",
    }


def amain(tmp_path, argv, name, event, gh=None, runner=None) -> int:
    env = env_for(tmp_path, name, event)
    args = cg.parse_args(argv)
    runner = runner or FakeRunner([], make_result())
    return asyncio.run(cg.amain(args, gh or FakeGitHub(), env, runner, tmp_path / "t"))


START_CASES = [
    ("review", "pull_request", REVIEW_EVENT, ("comment", (7, "Reviewing abcdef1…"))),
    ("code-review", "issue_comment", CODE_REVIEW_EVENT, ("comment", (7, "Working"))),
    (
        "mention",
        "pull_request_review_comment",
        comment_event(in_reply_to_id=4),
        ("reply", (7, 4, "Working")),
    ),
]


@pytest.mark.parametrize("job_name,name,event,posted", START_CASES)
def test_amain_start(tmp_path, capsys, job_name, name, event, posted):
    gh = FakeGitHub()
    assert amain(tmp_path, ["start", job_name], name, event, gh) == 0
    [(kind, fields)] = gh.posted
    assert (kind, fields[:-1]) == (posted[0], posted[1][:-1])
    assert fields[-1].startswith(posted[1][-1])
    assert fields[-1].endswith("[run](https://gh/o/r/actions/runs/5)")
    assert capsys.readouterr().out == "1\n"


@pytest.mark.parametrize(
    "argv,name,event,compares,diff_file",
    [
        (["run", "review"], "pull_request", REVIEW_EVENT, [(BASE, SHA)], True),
        (
            ["run", *CODE_REVIEW_ARGV],
            "issue_comment",
            CODE_REVIEW_EVENT,
            [(BASE, SHA)],
            False,
        ),
        (
            ["run", "review", "--head-sha", SHA, "--base-sha", BASE],
            "issue_comment",
            CODE_REVIEW_EVENT,
            [(BASE, SHA)],
            True,
        ),
        (["run", "mention"], "issue_comment", ISSUE_COMMENT_EVENT, [], False),
    ],
)
def test_amain_run(tmp_path, argv, name, event, compares, diff_file):
    gh = FakeGitHub()
    runner = FakeRunner([("comment", {"body": "s"})], make_result())
    assert amain(tmp_path, argv, name, event, gh, runner) == 0
    assert (gh.head_fetches, gh.live_diffs, gh.compares) == (0, 0, compares)
    assert (tmp_path / "t" / "pr.diff").exists() == diff_file
    [j] = runner.jobs
    rec = cg.parse_record(
        json.loads((tmp_path / "t" / "actions.json").read_text()), argv[1]
    )
    expected = None if argv[1] == "mention" else SHA
    assert rec.summary == "s" and rec.head_sha == expected == j.record.head_sha


def test_amain_run_workdir_splits_tmp(tmp_path):
    work = tmp_path / "pr"
    argv = ["run", "review", "--workdir", str(work)]
    assert amain(tmp_path, argv, "pull_request", REVIEW_EVENT) == 0
    assert (work / "tmp" / "pr.diff").read_text() == "diff"
    assert not (tmp_path / "t" / "pr.diff").exists()
    assert (tmp_path / "t" / "actions.json").exists()
    assert not (work / "tmp" / "actions.json").exists()


def test_amain_run_diff_unavailable(tmp_path):
    gh = FakeGitHub(diff=failed(406, "too large"))
    assert amain(tmp_path, ["run", "review"], "pull_request", REVIEW_EVENT, gh) == 0
    rec = cg.parse_record(
        json.loads((tmp_path / "t" / "actions.json").read_text()), "review"
    )
    assert rec.diff_unavailable is True
    assert not (tmp_path / "t" / "pr.diff").exists()


def apply_argv(job_name: str, path: Path) -> list[str]:
    return [
        "apply",
        job_name,
        "--status-id",
        "3",
        "--run-result",
        "success",
        "--record",
        str(path),
    ]


def test_amain_apply_review_uses_record_sha(tmp_path):
    path = tmp_path / "actions.json"
    cg.write_record(
        path, record([cg.InlineComment("a.rs", 1, "x")], head_sha="cafe123")
    )
    gh = FakeGitHub()
    assert (
        amain(tmp_path, apply_argv("review", path), "pull_request", REVIEW_EVENT, gh)
        == 0
    )
    assert gh.posted == [("inline", (7, "a.rs", 1, "x", "cafe123"))]
    assert gh.edits[0][0] == cg.Posted(3, False)
    assert gh.edits[0][1].startswith("### Review of #7 at cafe123")


def test_amain_apply_code_review(tmp_path):
    path = tmp_path / "actions.json"
    cg.write_record(path, record([cg.Reply(5, "r")], job_name="code-review"))
    gh = FakeGitHub()
    argv = apply_argv("code-review", path)
    assert amain(tmp_path, argv, "issue_comment", CODE_REVIEW_EVENT, gh) == 0
    assert gh.posted == [("reply", (7, 5, "r"))]
    assert gh.edits[0][1].startswith(HEADER)


def test_amain_apply_mention_edits_thread_reply(tmp_path):
    path = tmp_path / "actions.json"
    cg.write_record(path, record(job_name="mention", summary="answer", head_sha=None))
    gh = FakeGitHub()
    event = comment_event(in_reply_to_id=4)
    argv = apply_argv("mention", path)
    assert amain(tmp_path, argv, "pull_request_review_comment", event, gh) == 0
    assert gh.edits == [(cg.Posted(3, True), f"answer\n\n{FOOT}")]


def test_amain_apply_missing_record(tmp_path):
    gh = FakeGitHub()
    argv = apply_argv("review", tmp_path / "absent.json")
    assert amain(tmp_path, argv, "pull_request", REVIEW_EVENT, gh) == 1
    assert gh.edits[0][1].startswith("Review failed: no actions file")


def test_marked_body_fits_github_limit():
    body = cg.marked("x" * cg.MAX_BODY)
    assert body.startswith(cg.MARKER) and len(body) == 65536
