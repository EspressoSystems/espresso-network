"""Tests for eval/eval.py: its GitHub never writes."""

import asyncio
import importlib.util
import json
from pathlib import Path

import pytest

import claude_gh as cg

_spec = importlib.util.spec_from_file_location(
    "claude_eval", Path(__file__).parents[1] / "eval" / "eval.py"
)
assert _spec and _spec.loader
ev = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ev)

WRITES = {
    "inline_comment": (7, "a.rs", 1, "body", "sha"),
    "reply": (7, 1, "body"),
    "comment": (7, "body"),
    "edit_comment": (cg.Posted(1, False), "body"),
    "resolve_thread": ("T_1",),
    "thread": ("T_1",),
}


class Untouchable:
    def __getattr__(self, name):
        raise AssertionError(f"inner GitHub called: {name}")


class Reads:
    async def pr_view(self, pr):
        return f"view {pr}"


@pytest.mark.parametrize("name", WRITES)
def test_writes_raise_without_touching_inner(name):
    gh = ev.ReadOnlyGitHub(Untouchable())
    with pytest.raises(ev.ReadOnly):
        asyncio.run(getattr(gh, name)(*WRITES[name]))


def test_every_protocol_write_is_covered():
    reads = {
        "pr_diff",
        "compare_diff",
        "pr_view",
        "issue_view",
        "head_sha",
        "pr_payload",
    }
    methods = {m for m in vars(cg.GitHub) if not m.startswith("_")}
    assert methods - reads == set(WRITES)


def test_reads_delegate():
    assert asyncio.run(ev.ReadOnlyGitHub(Reads()).pr_view(7)) == "view 7"


def test_section_lists_wrong_and_unjudged(tmp_path):
    rec = cg.Record("review", "opus", [], "- **nit**: x", "<sub>f</sub>", None)
    rec.candidates = [
        cg.Candidate("a.rs", 1, "kept one", "kept", ""),
        cg.Candidate("b.rs", 2, "wrong one", "wrong", "`if x`"),
        cg.Candidate("c.rs", 3, "dup one", "duplicate", "c1"),
        cg.Candidate("d.rs", 4, "open one"),
    ]
    (tmp_path / "actions.json").write_text(json.dumps(cg.to_json(rec)))
    (tmp_path / "exit").write_text("0\n")
    case = {"fixed_by": [9], "bug": "b"}
    assert ev.section(7, case, tmp_path) == (
        "## #7 (fixed by #9)\n\nBug: b\n\n"
        "f · exit 0 · error None\n\n"
        "Findings:\n- **nit**: x\n\n"
        "Wrong candidates:\n- b.rs:2 wrong one\n  - evidence: `if x`\n\n"
        "Unjudged candidates:\n- d.rs:4 open one\n"
    )
    assert ev.result_line(7, tmp_path) == (
        "7: exit=0 f findings=1 wrong=1 unjudged=1 error=None"
    )


def test_findings_skip_checked_bullets():
    summary = (
        "- **blocking**: a (x.rs:1)\n"
        "- **nit**: b (y.rs:2)\n"
        "Checked:\n"
        "- **Callers:** all of them."
    )
    rec = cg.Record("review", "opus", [], summary, "<sub>f</sub>", None)
    assert ev.findings(rec) == ["- **blocking**: a (x.rs:1)", "- **nit**: b (y.rs:2)"]
