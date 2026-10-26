"""Tests for `scripts/release`: pure core first, then the `cmd_*` entry points.

Side effects go through a `FakeRunner` that maps argv prefixes to canned stdout and records every
call, so a test can assert that a refused operation touched neither git nor gh.

    just py::test
"""

import argparse
import contextlib
import importlib.util
import io
import json
import os
import subprocess
import tempfile
import unittest
import unittest.mock
from importlib.machinery import SourceFileLoader
from pathlib import Path

SCRIPT = Path(__file__).with_name("release")
_spec = importlib.util.spec_from_loader(
    "release", SourceFileLoader("release", str(SCRIPT))
)
assert _spec is not None
rel = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(rel)


REPO = "espressosystems/espresso-network"
WRITERS = {"alice"}
PERMISSION = ("gh", "api", "repos/{owner}/{repo}/collaborators/alice/permission")


class FakeRunner:
    """Maps an argv prefix to canned stdout; records every call for later assertion."""

    def __init__(self, responses: dict[tuple[str, ...], str]):
        self.responses = responses
        self.calls: list[list[str]] = []

    def __call__(self, argv: list[str]) -> str:
        self.calls.append(argv)
        for prefix, output in self.responses.items():
            if tuple(argv[: len(prefix)]) == prefix:
                return output
        raise AssertionError(f"unexpected command: {argv!r}")

    def ran(self, *prefix: str) -> bool:
        return any(tuple(call[: len(prefix)]) == prefix for call in self.calls)


def ls_remote(*tags: str) -> str:
    return "".join(f"{'a' * 40}\trefs/tags/{tag}\n" for tag in tags)


def version(major=0, minor=6, phase=0) -> "rel.Version":
    return rel.Version(major, minor, phase)


def release(tag="0.6.3.1", title=None, body="") -> "rel.ReleaseInfo":
    return rel.ReleaseInfo(tag, title or tag, body)


def release_row(tag, name=None, body="") -> str:
    row = {"tag_name": tag, "name": name, "body": body}
    return json.dumps(row)


def commit(sha="a" * 40, subject="fix: thing", pr=None) -> "rel.Commit":
    return rel.Commit(sha, subject, pr)


def state(**overrides) -> "rel.TrackerState":
    defaults = {
        "version": version(),
        "anchor": None,
        "anchor_reachable": True,
        "head_main": "m" * 40,
        "head_branch": "b" * 40,
        "tags": [],
        "releases": [],
        "main_commits": [],
        "branch_commits": [],
        "landed_on_branch": set(),
        "landed_on_main": set(),
        "marks": {},
        "backport_prs": {},
        "experimental_branches": [],
        "branch_tags": {},
        "fragment_counts": {},
    }
    defaults.update(overrides)
    return rel.TrackerState(**defaults)


# REQ:release-version-parse


class VersionParse(unittest.TestCase):
    def test_release_version_parse_ok(self):
        self.assertEqual(rel.parse_version("0.6.0"), version())
        self.assertEqual(rel.parse_version("Release 0.6.0"), version())

    def test_release_version_parse_fails(self):
        for text in ("0.6", "v0.6.0", "0.6.0.1"):
            self.assertIsNone(rel.parse_version(text), text)


# REQ:release-branch-parse


class BranchParse(unittest.TestCase):
    def test_release_branch_parse_ok(self):
        self.assertEqual(
            rel.parse_release_branch("release-0.6.0--topic"), (version(), True)
        )
        self.assertEqual(rel.parse_release_branch("release-0.6.0"), (version(), False))
        self.assertIsNone(rel.parse_release_branch("main"))


# REQ:release-next-version


class NextVersion(unittest.TestCase):
    def test_release_next_versions_ok(self):
        existing = [version(0, 6, 1), version(0, 6, 3), version(0, 5, 9)]
        self.assertEqual(
            rel.next_versions(existing),
            {
                version(0, 6, 4),
                version(0, 5, 10),
                version(0, 7, 0),
                version(1, 0, 0),
            },
        )
        self.assertEqual(rel.next_versions([]), set())

    def test_release_next_versions_two_lines_ok(self):
        existing = [version(0, 6, 3), version(0, 7, 0)]
        self.assertEqual(
            rel.next_versions(existing),
            {
                version(0, 6, 4),
                version(0, 7, 1),
                version(0, 8, 0),
                version(1, 0, 0),
            },
        )

    def test_release_resolve_version_default_phase_ok(self):
        self.assertEqual(
            rel.resolve_version(None, [version(0, 6, 3)]), version(0, 6, 4)
        )

    def test_release_resolve_version_default_phase_two_lines_ok(self):
        existing = [version(0, 6, 3), version(0, 7, 0)]
        self.assertEqual(rel.resolve_version(None, existing), version(0, 7, 1))

    def test_release_resolve_version_explicit_ok(self):
        self.assertEqual(
            rel.resolve_version(version(0, 7, 0), [version(0, 6, 3)]), version(0, 7, 0)
        )

    def test_release_resolve_version_bootstrap_ok(self):
        self.assertEqual(rel.resolve_version(version(0, 6, 0), []), version(0, 6, 0))

    def test_release_resolve_version_fails(self):
        with self.assertRaises(ValueError):
            rel.resolve_version(version(0, 6, 5), [version(0, 6, 3)])
        with self.assertRaises(ValueError):
            rel.resolve_version(version(0, 6, 2), [version(0, 6, 3)])
        with self.assertRaises(ValueError):
            rel.resolve_version(None, [])

    def test_release_cmd_next_version_ok(self):
        heads = (
            "a" * 40
            + " refs/heads/release-0.6.3\n"
            + "b" * 40
            + " refs/heads/release-0.6.3--x\n"
        )
        runner = FakeRunner({("git", "ls-remote", "--heads"): heads})
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rel.cmd_next_version(
                argparse.Namespace(version=None), rel.Git(runner), rel.Gh(runner)
            )
        self.assertEqual((code, out.getvalue()), (0, "0.6.4\n"))
        code = rel.cmd_next_version(
            argparse.Namespace(version="0.6.9"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 1)


# REQ:release-next-tag


class NextTag(unittest.TestCase):
    def test_release_next_tag_gap_ok(self):
        self.assertEqual(
            rel.next_tag(version(), ["0.6.0.0", "0.6.0.3"]), rel.Tag(version(), 4)
        )

    def test_release_next_tag_empty_ok(self):
        self.assertEqual(rel.next_tag(version(), []), rel.Tag(version(), 0))

    def test_release_next_tag_other_version_ok(self):
        self.assertEqual(
            rel.next_tag(version(), ["0.7.0.5", "not-a-tag"]), rel.Tag(version(), 0)
        )


# REQ:release-explicit-tag


class ExplicitTag(unittest.TestCase):
    def test_release_explicit_tag_ok(self):
        self.assertEqual(
            rel.validate_explicit_tag("0.6.0.4", version(), ["0.6.0.0"]),
            rel.Tag(version(), 4),
        )

    def test_release_explicit_tag_fails(self):
        with self.assertRaisesRegex(ValueError, "does not belong to"):
            rel.validate_explicit_tag("0.5.0.1", version(), [])
        with self.assertRaisesRegex(ValueError, "exists"):
            rel.validate_explicit_tag("0.6.0.3", version(), ["0.6.0.3"])


# REQ:release-command-parse


class CommandParse(unittest.TestCase):
    def test_release_command_parse_ok(self):
        self.assertEqual(rel.parse_command("/tag"), rel.Command("tag", None))
        self.assertEqual(
            rel.parse_command("/tag 0.6.0.4"), rel.Command("tag", "0.6.0.4")
        )
        self.assertEqual(
            rel.parse_command("/done abc1234"), rel.Command("done", "abc1234")
        )

    def test_release_command_parse_text_ok(self):
        self.assertEqual(
            rel.parse_command("/tag\r\nUrgency: High\n\n- env X\n"),
            rel.Command("tag", None, "Urgency: High\n\n- env X"),
        )

    def test_release_command_bot_marker_fails(self):
        for body in (
            "/publish 0.7.0.4 decaf\n<!-- /publish:decaf -->",
            "/tag\nUrgency: High <!-- operator-notes -->",
        ):
            self.assertIsNone(rel.parse_command(body), body)

    def test_release_bad_command_marker_message_only_for_tag_publish_text(self):
        usage = "`/backport <PR>`"
        self.assertEqual(
            rel.bad_command("/backport bad\n<!-- x -->", usage),
            f"comment '/backport bad\\n<!-- x -->' is not {usage}",
        )
        self.assertIn(
            "must not contain",
            rel.bad_command("/tag\nUrgency <!-- x -->", "a /tag command"),
        )

    def test_release_command_parse_mid_text_fails(self):
        self.assertIsNone(rel.parse_command("please /tag this"))


# REQ:release-sha-validation


class ShaValidation(unittest.TestCase):
    def test_release_sha_validation_too_short_fails(self):
        self.assertIsNone(rel.parse_command("/done abc"))

    def test_release_sha_validation_lowercases_ok(self):
        self.assertEqual(
            rel.parse_command("/done ABCDEF0"), rel.Command("done", "abcdef0")
        )


# REQ:release-backport-command


class BackportCommand(unittest.TestCase):
    def test_release_backport_parse_ok(self):
        self.assertEqual(
            rel.parse_command("/backport 123"), rel.Command("backport", "123")
        )
        self.assertEqual(
            rel.parse_command("/backport #123\r\n"), rel.Command("backport", "123")
        )
        self.assertIsNone(rel.parse_command("/backport abc"))
        self.assertIsNone(rel.parse_command("/backport"))
        self.assertIsNone(rel.parse_command("/backport 1 2"))

    def test_release_backport_not_a_mark_ok(self):
        marks = rel.marks_from_comments(
            [rel.Comment("/backport 123", "alice")], WRITERS
        )
        self.assertEqual(marks, {})

    def _runner(self) -> FakeRunner:
        return FakeRunner(
            {
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("gh", "repo", "view"): REPO,
                ("gh", "workflow", "run"): "",
                ("gh", "issue", "comment"): "",
                PERMISSION: "write",
                (
                    "gh",
                    "api",
                    "repos/{owner}/{repo}/collaborators/mallory/permission",
                ): "read",
            }
        )

    def test_release_cmd_backport_ok(self):
        runner = self._runner()
        args = argparse.Namespace(
            issue=42, comment="/backport #123", actor="alice", run_url=None
        )
        self.assertEqual(rel.cmd_backport(args, rel.Git(runner), rel.Gh(runner)), 0)
        self.assertIn(
            [
                "gh",
                "workflow",
                "run",
                "backport.yml",
                "-f",
                "pr_number=123",
                "-f",
                "target_branch=release-0.6.0",
            ],
            runner.calls,
        )
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("Backporting [#123]", comment[-1])
        self.assertIn("@alice", comment[-1])

    def test_release_cmd_backport_fails(self):
        runner = self._runner()
        args = argparse.Namespace(
            issue=42, comment="/backport abc", actor="alice", run_url=None
        )
        self.assertEqual(rel.cmd_backport(args, rel.Git(runner), rel.Gh(runner)), 1)
        self.assertFalse(runner.ran("gh", "workflow", "run"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("/backport failed", comment[-1])

    def test_release_cmd_backport_non_writer_fails(self):
        runner = self._runner()
        args = argparse.Namespace(
            issue=42, comment="/backport 123", actor="mallory", run_url=None
        )
        self.assertEqual(rel.cmd_backport(args, rel.Git(runner), rel.Gh(runner)), 1)
        self.assertFalse(runner.ran("gh", "workflow", "run"))
        self.assertFalse(runner.ran("gh", "issue", "comment"))


# REQ:release-marks-replay


class MarksReplay(unittest.TestCase):
    def test_release_marks_replay_ok(self):
        comments = [
            rel.Comment("/done abc1234", "alice"),
            rel.Comment("/skip def4567", "alice"),
            rel.Comment("/unmark abc1234", "alice"),
        ]
        self.assertEqual(
            rel.marks_from_comments(comments, WRITERS), {"def4567": "skip"}
        )

    def test_release_marks_done_with_html_comment_later_line_ok(self):
        comments = [rel.Comment("/done abc1234\nnote\n<!-- hidden -->", "alice")]
        self.assertEqual(
            rel.marks_from_comments(comments, WRITERS), {"abc1234": "done"}
        )

    def test_release_command_authors_ok(self):
        comments = [rel.Comment("/done abc1234", "alice"), rel.Comment("hi", "bob")]
        self.assertEqual(rel.command_authors(comments), {"alice"})

    def test_release_marks_replay_nonmember_fails(self):
        comments = [rel.Comment("/done abc1234", "mallory")]
        self.assertEqual(rel.marks_from_comments(comments, WRITERS), {})

    def test_release_marks_replay_unmark_prefix_symmetric_ok(self):
        self.assertEqual(
            rel.marks_from_comments(
                [
                    rel.Comment("/done abc1234", "alice"),
                    rel.Comment("/unmark abc1234567890", "alice"),
                ],
                WRITERS,
            ),
            {},
        )
        self.assertEqual(
            rel.marks_from_comments(
                [
                    rel.Comment("/done abc1234567890", "alice"),
                    rel.Comment("/unmark abc1234", "alice"),
                ],
                WRITERS,
            ),
            {},
        )
        self.assertEqual(
            rel.marks_from_comments(
                [
                    rel.Comment("/done abc1234", "alice"),
                    rel.Comment("/unmark fed1234", "alice"),
                ],
                WRITERS,
            ),
            {"abc1234": "done"},
        )


# REQ:release-log-parse


class Landed(unittest.TestCase):
    def test_release_match_key_ok(self):
        self.assertEqual(
            rel.match_key("[Backport release-0.6.0] fix(x): thing  (#4867)"),
            "fix(x): thing",
        )
        self.assertEqual(rel.match_key("fix(x): thing (#4865)"), "fix(x): thing")

    def test_release_landed_ok(self):
        on_main = [
            commit("1" * 40, "fix(x): thing (#4865)", 4865),
            commit("2" * 40, "feat: other (#4870)", 4870),
            commit("3" * 40, "chore: only on main (#4871)", 4871),
            commit("4" * 40, "fix: by patch id (#4872)", 4872),
        ]
        on_branch = [
            commit("a" * 40, "[Backport release-0.6.0] fix(x): thing (#4900)", 4900),
            commit("b" * 40, "feat: other (#4870)", 4870),
            commit("c" * 40, "fix: by patch id, retitled (#4999)", 4999),
        ]
        result = rel.landed(on_main, on_branch, {"4" * 40}, set())
        self.assertEqual(result, {"1" * 40, "2" * 40, "4" * 40})
        self.assertEqual(
            rel.landed(on_branch, on_main, set(), set()), {"a" * 40, "b" * 40}
        )

    def test_release_landed_merged_backport_ok(self):
        on_main = [commit("1" * 40, "chore: only on main (#4871)", 4871)]
        self.assertEqual(rel.landed(on_main, [], set(), {4871}), {"1" * 40})
        self.assertEqual(rel.landed(on_main, [], set(), {4870}), set())

    def test_release_landed_title_requires_backport_prefix_ok(self):
        commits = [commit("1" * 40, "fix: typo (#1)", 1)]
        other = [commit("2" * 40, "fix: typo (#2)", 2)]
        self.assertEqual(rel.landed(commits, other, set(), set()), set())

    def test_release_landed_title_backport_prefix_ticks_ok(self):
        commits = [commit("3" * 40, "[Backport release-0.6.0] fix: typo (#3)", 3)]
        other = [commit("1" * 40, "fix: typo (#1)", 1)]
        self.assertEqual(rel.landed(commits, other, set(), set()), {"3" * 40})


class LogParse(unittest.TestCase):
    def test_release_log_parse_ok(self):
        text = "aaaa\x00Fix the thing (#42)\nbbbb\x00Merge pull request #7 from x/y"
        commits = rel.parse_git_log(text)
        self.assertEqual([c.pr for c in commits], [42, 7])
        self.assertEqual([c.sha for c in commits], ["aaaa", "bbbb"])


# REQ:release-backport-index


class BackportIndex(unittest.TestCase):
    def test_release_backport_index_ok(self):
        items = [
            {
                "number": 20,
                "state": "MERGED",
                "url": "https://example/pr/20",
                "headRefName": "backport-10-to-release-0.6.0",
            },
            {
                "number": 21,
                "state": "OPEN",
                "url": "https://example/pr/21",
                "headRefName": "some-other-branch",
            },
        ]
        index = rel.backport_index(items, "release-0.6.0")
        self.assertEqual(
            index, {10: rel.BackportPr(20, "MERGED", "https://example/pr/20")}
        )

    def test_release_backport_index_retried(self):
        head = "backport-10-to-release-0.6.0"
        items = [
            {"number": 30, "state": "MERGED", "url": "u30", "headRefName": head},
            {"number": 20, "state": "CLOSED", "url": "u20", "headRefName": head},
        ]
        for order in (items, items[::-1]):
            index = rel.backport_index(order, "release-0.6.0")
            self.assertEqual(index, {10: rel.BackportPr(30, "MERGED", "u30")})

    def test_release_backport_index_duplicate_after_merge(self):
        head = "backport-10-to-release-0.6.0"
        items = [
            {"number": 40, "state": "CLOSED", "url": "u40", "headRefName": head},
            {"number": 30, "state": "MERGED", "url": "u30", "headRefName": head},
        ]
        index = rel.backport_index(items, "release-0.6.0")
        self.assertEqual(index, {10: rel.BackportPr(30, "MERGED", "u30")})


# REQ:release-refresh-targets


class RefreshTargets(unittest.TestCase):
    def test_release_refresh_targets_ok(self):
        open_trackers = [(1, "Release 0.6.0"), (2, "Release 0.7.0")]
        self.assertEqual(
            rel.refresh_targets("refs/heads/release-0.6.0--topic", None, open_trackers),
            [1],
        )
        self.assertEqual(
            rel.refresh_targets("refs/heads/main", None, open_trackers), [1, 2]
        )


# REQ:release-render-notes


class RenderNotes(unittest.TestCase):
    def test_release_render_notes_ok(self):
        body = rel.render_body(state(), "Some human note.\n", REPO)
        self.assertTrue(body.endswith(f"{rel.SENTINEL}\nSome human note.\n"))
        self.assertEqual(rel.split_human_notes(body), "Some human note.\n")


# REQ:release-checklist-cap


class ChecklistCap(unittest.TestCase):
    def test_release_checklist_cap_large_body_ok(self):
        main_commits = [
            commit(sha=f"{i:040x}", subject=f"feat: thing {i} (#{i})", pr=i)
            for i in range(1000)
        ]
        branch_commits = [
            commit(
                sha=f"{i:040x}", subject=f"fix: other {i} (#{i + 2000})", pr=i + 2000
            )
            for i in range(1000)
        ]
        body = rel.render_body(
            state(
                anchor="a" * 40,
                tags=[("0.6.0.0", "2026-01-01", "a" * 40)],
                main_commits=main_commits,
                branch_commits=branch_commits,
            ),
            "",
            REPO,
        )
        self.assertLess(len(body), 65536)
        self.assertRegex(body, r"_… and \d+ more( \(\d+ ported hidden\))?_")

    def test_release_checklist_cap_landed_hidden_ok(self):
        commits = [
            commit(sha=f"{i:08x}" + "0" * 32, subject=f"fix: thing {i}")
            for i in range(200)
        ]
        landed = {c.sha for c in commits[:100]}
        rendered = rel.render_checklist(
            commits, "a" * 40, True, "h" * 40, {}, landed, {}, REPO
        )
        for c in commits[100:]:
            self.assertIn(f"[`{c.sha[:8]}`]", rendered)
        for c in commits[:100]:
            self.assertNotIn(f"[`{c.sha[:8]}`]", rendered)
        self.assertIn("_… and 100 more (100 ported hidden)_", rendered)


# REQ:release-comments-paginate


class CommentsPaginate(unittest.TestCase):
    def test_release_comments_paginate_ok(self):
        runner = FakeRunner(
            {
                ("gh", "api"): (
                    '{"body": "second", "login": "alice", '
                    '"created_at": "2026-01-02T00:00:00Z"}\n'
                    '{"body": "first", "login": "bob", '
                    '"created_at": "2026-01-01T00:00:00Z"}\n'
                )
            }
        )
        comments = rel.Gh(runner).issue_comments(5)
        self.assertEqual([c.body for c in comments], ["first", "second"])
        self.assertIn("--paginate", runner.calls[0])


# REQ:release-tag-refuses-existing


class TagRefusesExisting(unittest.TestCase):
    def test_release_tag_refuses_existing_fails(self):
        runner = FakeRunner(
            {
                ("gh", "repo"): REPO,
                ("gh", "issue", "list"): "[]",
                ("git", "fetch"): "",
                ("git", "tag", "--list", "0.6.0.*"): "0.6.0.3\n",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            issue=None,
            branch="release-0.6.0",
            comment=None,
            explicit="0.6.0.3",
            actor="someone",
            run_url=None,
            dry_run=False,
        )
        code = rel.cmd_tag(args, git, gh)
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("git", "push"))
        self.assertFalse(runner.ran("gh", "release", "create"))
        self.assertFalse(runner.ran("git", "tag", "-a"))


# REQ:release-tag-refuses-no-remote


class TagRefusesPushWithoutRemote(unittest.TestCase):
    def test_release_tag_refuses_push_without_remote_fails(self):
        runner = FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): "[]",
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("git", "tag", "--list", "[0-9]*"): "0.5.9.1\n",
                ("git", "diff"): "",
                ("git", "rev-parse", "release-0.6.0"): "c" * 40,
            }
        )
        git, gh = rel.Git(runner, remote=None), rel.Gh(runner)
        args = argparse.Namespace(
            issue=None,
            branch="release-0.6.0",
            comment=None,
            explicit=None,
            actor="alice",
            run_url=None,
            dry_run=False,
        )
        with self.assertRaises(RuntimeError):
            rel.cmd_tag(args, git, gh)
        self.assertFalse(runner.ran("git", "push"))
        self.assertFalse(runner.ran("gh", "release", "create"))


# REQ:release-tag-bad-comment


class CmdTagBadComment(unittest.TestCase):
    def test_release_cmd_tag_bad_comment_fails(self):
        runner = FakeRunner({("gh", "repo", "view"): REPO, PERMISSION: "write"})
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            issue=None,
            branch="release-0.6.0",
            comment="/tag please 0.5.0.9",
            explicit=None,
            actor="alice",
            run_url=None,
            dry_run=False,
        )
        code = rel.cmd_tag(args, git, gh)
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("git", "tag"))
        self.assertFalse(runner.ran("git", "push"))
        self.assertFalse(runner.ran("gh", "release", "create"))

    def test_release_cmd_tag_bot_marker_comments_fails(self):
        runner = FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("gh", "issue", "comment"): "",
                PERMISSION: "write",
            }
        )
        args = argparse.Namespace(
            issue=42,
            branch=None,
            comment="/tag\n<!-- operator-notes -->",
            explicit=None,
            actor="alice",
            run_url=None,
            dry_run=False,
        )
        self.assertEqual(rel.cmd_tag(args, rel.Git(runner), rel.Gh(runner)), 1)
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("must not contain", comment[-1])
        self.assertFalse(runner.ran("git", "tag"))

    def test_release_cmd_tag_non_writer_fails(self):
        runner = FakeRunner(
            {
                (
                    "gh",
                    "api",
                    "repos/{owner}/{repo}/collaborators/mallory/permission",
                ): "read"
            }
        )
        args = argparse.Namespace(
            issue=42,
            branch=None,
            comment="/tag",
            explicit=None,
            actor="mallory",
            run_url=None,
            dry_run=False,
        )
        self.assertEqual(rel.cmd_tag(args, rel.Git(runner), rel.Gh(runner)), 1)
        self.assertEqual(len(runner.calls), 1)


# REQ:release-tag-happy-path


class CmdTagHappyPath(unittest.TestCase):
    def test_release_cmd_tag_happy_path_ok(self):
        sha = "c" * 40
        runner = FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("git", "fetch"): "",
                ("git", "tag", "--list", "0.6.0.*"): "0.6.0.0\n0.6.0.1\n",
                ("git", "rev-parse", "origin/release-0.6.0"): sha,
                ("git", "diff"): "",
                ("git", "tag", "-a"): "",
                ("git", "push", "origin", "refs/tags/0.6.0.2"): "",
                ("gh", "workflow", "run"): "",
                ("gh", "issue", "comment"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            issue=42,
            branch=None,
            comment=None,
            explicit=None,
            actor="alice",
            run_url=None,
            dry_run=False,
        )
        code = rel.cmd_tag(args, git, gh)
        self.assertEqual(code, 0)
        self.assertIn(
            ["git", "tag", "-a", "0.6.0.2", sha, "-m", "Release 0.6.0.2"], runner.calls
        )
        self.assertIn(["git", "push", "origin", "refs/tags/0.6.0.2"], runner.calls)
        self.assertFalse(runner.ran("gh", "release"))
        self.assertTrue(runner.ran("gh", "workflow", "run", "build.yml"))
        self.assertTrue(
            runner.ran("gh", "workflow", "run", "update-release-tracker.yml")
        )
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("Tagged `0.6.0.2`", comment[-1])
        self.assertIn("Build runs", comment[-1])
        self.assertNotIn("/releases/tag/", comment[-1])

    def test_release_cmd_tag_dry_run_ok(self):
        runner = FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("git", "fetch"): "",
                ("git", "tag", "--list"): "0.6.0.0\n",
                ("git", "rev-parse", "origin/release-0.6.0"): "c" * 40,
                ("git", "diff"): "",
                PERMISSION: "write",
            }
        )
        args = argparse.Namespace(
            issue=42,
            branch=None,
            comment="/tag",
            explicit=None,
            actor="alice",
            run_url=None,
            dry_run=True,
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rel.cmd_tag(args, rel.Git(runner), rel.Gh(runner))
        self.assertEqual(code, 0)
        self.assertIn("# 0.6.0.1 at " + "c" * 40, out.getvalue())
        self.assertIn("Tagged `0.6.0.1`", out.getvalue())
        self.assertFalse(runner.ran("git", "tag", "-a"))
        self.assertFalse(runner.ran("gh", "release", "create"))
        self.assertFalse(runner.ran("gh", "issue", "comment"))

    def test_release_git_local_mode_ok(self):
        runner = FakeRunner(
            {
                ("git", "for-each-ref"): "a" * 40 + " refs/heads/release-0.6.0--x\n",
                ("git", "rev-parse", "release-0.6.0^{commit}"): "b" * 40,
            }
        )
        git = rel.Git(runner, remote=None)
        git.fetch(version(), ["main"])
        self.assertEqual(git.ref("main"), "origin/main")
        self.assertEqual(git.ref("release-0.6.0"), "release-0.6.0")
        self.assertEqual(git.resolve("release-0.6.0"), "b" * 40)
        self.assertEqual(
            git.ls_remote_heads("release-0.6.0--*"), [("release-0.6.0--x", "a" * 40)]
        )
        self.assertFalse(runner.ran("git", "fetch"))

    def test_release_git_fetch_version_tags_only_ok(self):
        runner = FakeRunner({("git", "fetch"): ""})
        rel.Git(runner).fetch(version(), ["main", "release-0.6.0"])
        self.assertEqual(
            runner.calls[-1],
            [
                "git",
                "fetch",
                "--quiet",
                "origin",
                "main",
                "release-0.6.0",
                "+refs/tags/0.6.0.*:refs/tags/0.6.0.*",
            ],
        )

    def test_release_gh_release_list_ok(self):
        lines = [
            release_row("0.6.1.0", "0.6.1.0", "a\r\nb"),
            release_row("0.6.1.3", "0.6.1.3 [mainnet]", None),
            release_row("0.7.0.0", ""),
        ]
        runner = FakeRunner({("gh", "api"): "\n".join(lines) + "\n"})
        self.assertEqual(
            rel.Gh(runner).release_list(),
            [
                release("0.6.1.0", body="a\nb"),
                release("0.6.1.3", "0.6.1.3 [mainnet]"),
                release("0.7.0.0"),
            ],
        )
        self.assertIn("--paginate", runner.calls[-1])

    def _fragment_runner(self, diff: str, shown: str, tags="0.6.0.0\n"):
        return FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("git", "fetch"): "",
                ("git", "tag", "--list", "0.6.0.*"): tags,
                ("git", "rev-parse", "origin/release-0.6.0"): "c" * 40,
                ("git", "diff"): diff,
                ("git", "show"): shown,
                ("git", "tag", "-a"): "",
                ("git", "push"): "",
                ("gh", "release", "create"): "",
                ("gh", "workflow", "run"): "",
                ("gh", "issue", "comment"): "",
                PERMISSION: "write",
            }
        )

    def _tag_args(self, comment="/tag"):
        return argparse.Namespace(
            issue=42,
            branch=None,
            comment=comment,
            explicit=None,
            actor="alice",
            run_url=None,
            dry_run=False,
        )

    def test_release_cmd_tag_text_fails(self):
        runner = self._fragment_runner("", "")
        args = self._tag_args("/tag\nUrgency: High")
        self.assertEqual(rel.cmd_tag(args, rel.Git(runner), rel.Gh(runner)), 1)
        self.assertFalse(runner.ran("git", "tag", "-a"))
        self.assertFalse(runner.ran("gh", "release", "create"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("/tag failed:", comment[-1])
        self.assertIn("doc/operator-notes/", comment[-1])

    def _assert_tag_blocked(self, runner, message, dry_run=False):
        args = self._tag_args()
        args.dry_run = dry_run
        self.assertEqual(rel.cmd_tag(args, rel.Git(runner), rel.Gh(runner)), 1)
        self.assertFalse(runner.ran("git", "tag", "-a"))
        self.assertFalse(runner.ran("git", "push"))
        self.assertFalse(runner.ran("gh", "workflow", "run"))
        comments = [c for c in runner.calls if c[:3] == ["gh", "issue", "comment"]]
        if dry_run:
            self.assertEqual(comments, [])
        else:
            self.assertIn("/tag failed:", comments[0][-1])
            self.assertIn(message, comments[0][-1])

    def test_release_cmd_tag_fragment_bot_marker_fails(self):
        runner = self._fragment_runner(
            "doc/operator-notes/a.md\0", "<!-- operator-notes -->"
        )
        self._assert_tag_blocked(runner, "doc/operator-notes/a.md")

    def test_release_cmd_tag_fragment_invalid_path_fails(self):
        runner = self._fragment_runner("doc/operator-notes/sub/a.md\0", "x")
        self._assert_tag_blocked(runner, "is not a fragment")

    def test_release_cmd_tag_first_tag_without_earlier_line_fails(self):
        runner = self._fragment_runner("", "", tags="")
        runner.responses[("git", "ls-remote")] = ls_remote("0.6.0.0")
        self._assert_tag_blocked(runner, "no earlier version tag")

    def test_release_cmd_tag_invalid_fragment_dry_run_fails(self):
        runner = self._fragment_runner("doc/operator-notes/sub/a.md\0", "x")
        self._assert_tag_blocked(runner, "is not a fragment", dry_run=True)


# REQ:release-tag-failure-reported


class TagFailurePostsComment(unittest.TestCase):
    def test_release_tag_failure_posts_comment_ok(self):
        runner = FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("git", "fetch"): "",
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("gh", "issue", "comment"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            issue=42,
            branch=None,
            comment=None,
            explicit="0.5.0.1",
            actor="alice",
            run_url=None,
            dry_run=False,
        )
        code = rel.cmd_tag(args, git, gh)
        self.assertEqual(code, 1)
        comment_call = next(
            call for call in runner.calls if call[:3] == ["gh", "issue", "comment"]
        )
        self.assertIn("does not belong to 0.6.0", comment_call[-1])

    def test_release_tag_dry_run_failure_does_not_comment_fails(self):
        runner = FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("git", "fetch"): "",
                ("git", "tag", "--list", "0.6.0.*"): "",
            }
        )
        args = argparse.Namespace(
            issue=42,
            branch=None,
            comment=None,
            explicit="0.5.0.1",
            actor="alice",
            run_url=None,
            dry_run=True,
        )
        code = rel.cmd_tag(args, rel.Git(runner), rel.Gh(runner))
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("gh", "issue", "comment"))

    def test_release_tag_write_failure_posts_comment_and_reraises_fails(self):
        sha = "c" * 40
        responses = {
            ("gh", "repo", "view"): REPO,
            ("gh", "issue", "view"): json.dumps({"title": "Release 0.6.0", "body": ""}),
            ("git", "fetch"): "",
            ("git", "tag", "--list", "0.6.0.*"): "",
            ("git", "ls-remote", "--tags"): ls_remote("0.5.9.1"),
            ("git", "rev-parse", "origin/release-0.6.0"): sha,
            ("git", "diff"): "",
            ("git", "tag", "-a"): "",
            ("git", "push", "origin", "refs/tags/0.6.0.0"): "",
            ("gh", "issue", "comment"): "",
        }
        calls: list[list[str]] = []

        def runner(argv: list[str]) -> str:
            calls.append(argv)
            if argv[:3] == ["gh", "workflow", "run"]:
                raise RuntimeError("gh workflow run boom")
            for prefix, output in responses.items():
                if tuple(argv[: len(prefix)]) == prefix:
                    return output
            raise AssertionError(f"unexpected command: {argv!r}")

        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            issue=42,
            branch=None,
            comment=None,
            explicit=None,
            actor="alice",
            run_url=None,
            dry_run=False,
        )
        with self.assertRaises(RuntimeError):
            rel.cmd_tag(args, git, gh)
        comment_call = next(
            call for call in calls if call[:3] == ["gh", "issue", "comment"]
        )
        self.assertIn("/tag failed:", comment_call[-1])


# REQ:release-refresh-write-no-write


class CmdRefreshWriteNoWrite(unittest.TestCase):
    def _responses(self, body_before: str) -> dict[tuple[str, ...], str]:
        return {
            ("gh", "repo", "view"): REPO,
            ("gh", "issue", "list"): "[]",
            ("gh", "issue", "view"): json.dumps(
                {"title": "Release 0.6.0", "body": body_before}
            ),
            ("gh", "pr", "list"): "[]",
            ("gh", "api", "repos/{owner}/{repo}/issues/42/comments"): "",
            ("gh", "api", "repos/{owner}/{repo}/releases?per_page=100"): "",
            ("gh", "issue", "edit"): "",
            ("git", "fetch"): "",
            ("git", "rev-parse", "origin/main"): "m" * 40,
            ("git", "rev-parse", "origin/release-0.6.0"): "b" * 40,
            ("git", "tag", "--list", "0.6.0.0"): "",
            ("git", "tag", "--list", "0.6.0.*"): "",
            ("git", "ls-remote", "--heads"): "",
        }

    def _refresh(self, body_before: str) -> FakeRunner:
        runner = FakeRunner(self._responses(body_before))
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            ref="refs/heads/main", issue=42, version=None, dry_run=False
        )
        code = rel.cmd_refresh(args, git, gh)
        self.assertEqual(code, 0)
        return runner

    def test_release_refresh_marks_only_from_writers_ok(self):
        responses = self._responses("")
        responses[("gh", "api", "repos/{owner}/{repo}/issues/42/comments")] = (
            '{"body": "/done abc1234", "login": "alice", "created_at": "1"}\n'
            '{"body": "/skip def4567", "login": "mallory", "created_at": "2"}\n'
            '{"body": "thanks", "login": "bob", "created_at": "3"}\n'
        )
        responses[PERMISSION] = "write"
        responses[
            ("gh", "api", "repos/{owner}/{repo}/collaborators/mallory/permission")
        ] = "read"
        runner = FakeRunner(responses)
        tracker, _body = rel.build_tracker_state(
            rel.Git(runner), rel.Gh(runner), 42, version()
        )
        self.assertEqual(tracker.marks, {"abc1234": "done"})
        self.assertFalse(
            runner.ran("gh", "api", "repos/{owner}/{repo}/collaborators/bob/permission")
        )

    def test_release_cmd_refresh_writes_on_change_ok(self):
        runner = self._refresh("")
        self.assertTrue(runner.ran("gh", "issue", "edit"))

    def test_release_cmd_refresh_skips_on_no_change_ok(self):
        rendered = rel.render_body(state(), "", REPO)
        for body_before in (rendered, rendered.replace("\n", "\r\n")):
            with self.subTest(crlf="\r\n" in body_before):
                runner = self._refresh(body_before)
                self.assertFalse(runner.ran("gh", "issue", "edit"))


# REQ:release-refresh-dry-run


class RefreshDryRun(unittest.TestCase):
    def test_release_refresh_dry_run_ok(self):
        runner = FakeRunner(
            {
                ("gh", "repo"): REPO,
                ("gh", "issue", "list"): "[]",
                ("git", "fetch"): "",
                ("git", "rev-parse", "origin/main"): "m" * 40,
                ("git", "rev-parse", "origin/release-0.6.0"): "b" * 40,
                ("git", "tag", "--list", "0.6.0.0"): "",
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("git", "ls-remote", "--heads"): "",
                ("gh", "pr", "list"): "[]",
                ("gh", "api", "repos/{owner}/{repo}/releases?per_page=100"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            ref="refs/heads/main",
            issue=None,
            version="0.6.0",
            dry_run=True,
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rel.cmd_refresh(args, git, gh)
        self.assertEqual(code, 0)
        self.assertIn("# Release 0.6.0", out.getvalue())
        self.assertFalse(runner.ran("gh", "issue", "edit"))
        self.assertFalse(runner.ran("gh", "issue", "comment"))


# REQ:release-branch-tags


def tag(patch, minor=6) -> "rel.Tag":
    return rel.Tag(version(0, minor, 3), patch)


class FirstTags(unittest.TestCase):
    def test_release_first_tags_lowest_patch_ok(self):
        c1, c2, c3 = (commit(sha=str(i) * 40) for i in (1, 2, 3))
        contained = [
            (tag(2), [c1, c2, c3]),
            (tag(1), [c1, c2]),
            (tag(0), [c1]),
        ]
        self.assertEqual(
            rel.first_tags(contained),
            {c1.sha: "0.6.3.0", c2.sha: "0.6.3.1", c3.sha: "0.6.3.2"},
        )

    def test_release_first_tags_untagged_absent_ok(self):
        self.assertEqual(rel.first_tags([(tag(0), [])]), {})


class BranchTagColumn(unittest.TestCase):
    def render(self, commits, tag_of):
        return rel.render_checklist(
            commits, "a" * 40, True, "h" * 40, {}, set(), {}, REPO, tag_of
        )

    def test_release_branch_tag_column_ok(self):
        tagged, untagged = commit(sha="1" * 40), commit(sha="2" * 40)
        rendered = self.render([tagged, untagged], {tagged.sha: "0.6.3.1"})
        self.assertIn("| Commit | Tag | Backport |", rendered)
        self.assertIn("| `0.6.3.1` |  | fix: thing |", rendered)
        self.assertIn("| **untagged** |  | fix: thing |", rendered)

    def test_release_branch_tag_column_main_table_unchanged_ok(self):
        rendered = self.render([commit()], None)
        self.assertNotIn("Tag", rendered)

    def test_release_branch_untagged_summary_ok(self):
        commits = [commit(sha=str(i) * 40) for i in (1, 2, 3)]
        tags = [("0.6.0.0", "d", "a" * 40), ("0.6.0.2", "d", "b" * 40)]
        body = rel.render_body(
            state(
                anchor="a" * 40,
                tags=tags,
                branch_commits=commits,
                branch_tags={commits[0].sha: "0.6.0.1"},
            ),
            "",
            REPO,
        )
        self.assertIn("**2 commits since `0.6.0.2`**\n\n| ", body)
        body = rel.render_body(
            state(
                anchor="a" * 40,
                tags=tags,
                branch_commits=commits[:1],
                branch_tags={commits[0].sha: "0.6.0.1"},
            ),
            "",
            REPO,
        )
        self.assertIn("_All commits tagged._\n\n| ", body)
        body = rel.render_body(
            state(
                anchor="a" * 40,
                tags=tags,
                branch_commits=commits[:1],
            ),
            "",
            REPO,
        )
        self.assertIn("**1 commit since `0.6.0.2`**", body)

    def test_release_branch_untagged_summary_no_table_ok(self):
        body = rel.render_body(state(anchor="a" * 40), "", REPO)
        self.assertNotIn("commits since", body)
        self.assertNotIn("All commits tagged", body)


# REQ:release-build-tracker-state


class BuildTrackerStateAnchor(unittest.TestCase):
    def test_release_build_tracker_state_anchor_ok(self):
        anchor = "a" * 40
        main_ref, head_ref = "origin/main", "origin/release-0.6.0"
        cherry_sha, pr_match_sha, branch_sha = "1" * 40, "2" * 40, "3" * 40
        runner = FakeRunner(
            {
                ("git", "fetch"): "",
                ("git", "rev-parse", "origin/main"): "m" * 40,
                ("git", "rev-parse", "origin/release-0.6.0"): "b" * 40,
                ("git", "tag", "--list", "0.6.0.0"): "0.6.0.0\n",
                ("git", "diff"): "doc/operator-notes/a.md\0",
                ("git", "rev-parse", "0.6.0.0^{commit}"): anchor,
                ("git", "merge-base", "--is-ancestor"): "",
                (
                    "git",
                    "log",
                    "--no-merges",
                    "--format=%H%x00%s",
                    f"{anchor}..{main_ref}",
                ): (
                    f"{cherry_sha}\x00fix: thing (#10)\n"
                    f"{pr_match_sha}\x00feat: other (#20)\n"
                ),
                (
                    "git",
                    "log",
                    "--no-merges",
                    "--format=%H%x00%s",
                    f"{anchor}..{head_ref}",
                ): f"{branch_sha}\x00feat: other, retitled (#20)\n",
                ("git", "cherry", head_ref, main_ref, anchor): f"- {cherry_sha}\n",
                ("git", "cherry", main_ref, head_ref, anchor): "",
                ("gh", "pr", "list"): "[]",
                ("gh", "api", "repos/{owner}/{repo}/issues/7/comments"): "",
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("gh", "api", "repos/{owner}/{repo}/releases?per_page=100"): "",
                ("git", "ls-remote", "--heads"): "",
                ("git", "tag", "--list", "0.6.0.*"): (
                    f"0.6.0.1\x002026-09-22\x00\x00{branch_sha}\n"
                    f"0.6.0.0\x002026-09-21\x00\x00{anchor}\n"
                ),
                (
                    "git",
                    "log",
                    "--no-merges",
                    "--format=%H%x00%s",
                    f"{anchor}..{branch_sha}",
                ): f"{branch_sha}\x00feat: other, retitled (#20)\n",
                (
                    "git",
                    "log",
                    "--no-merges",
                    "--format=%H%x00%s",
                    f"{anchor}..{anchor}",
                ): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        state, _body_before = rel.build_tracker_state(git, gh, 7, version())
        self.assertEqual(state.branch_tags, {branch_sha: "0.6.0.1"})
        self.assertEqual(state.fragment_counts, {"0.6.0.1": rel.NoteCounts(1, 0)})
        self.assertEqual(
            [c.sha for c in state.main_commits], [cherry_sha, pr_match_sha]
        )
        self.assertEqual(state.landed_on_branch, {cherry_sha, pr_match_sha})
        self.assertTrue(state.anchor_reachable)


class TagLogReleases(unittest.TestCase):
    def test_release_tag_log_release_links_ok(self):
        tags = [
            ("0.6.3.0", "2026-09-21", "c" * 40),
            ("0.6.3.1", "2026-09-22", "d" * 40),
        ]
        body = rel.render_tag_log(tags, [release("0.6.3.1")], {}, REPO)
        self.assertIn("| GitHub release | Notes | Build |", body)
        self.assertIn(
            f"[release](https://github.com/{REPO}/releases/tag/0.6.3.1)", body
        )
        self.assertNotIn("pre-release", body)
        self.assertIn("build.yml?query=branch%3A0.6.3.0", body)
        self.assertIn("| - |", body)

    def test_release_tag_log_notes_column_ok(self):
        tags = [(f"0.6.3.{n}", "2026-09-21", "c" * 40) for n in range(3)]
        rows = rel.render_tag_log(
            tags, [], {"0.6.3.1": rel.NoteCounts(2, 0)}, REPO
        ).splitlines()[2:]
        self.assertIn("| - | - |", rows[0])
        self.assertIn("| - | 2 |", rows[1])
        self.assertIn("| - | - |", rows[2])

    def test_release_fragment_counts_ok(self):
        runner = FakeRunner({("git", "diff"): "doc/operator-notes/a.md\0"})
        tags = [(f"0.6.3.{n}", "d", "c" * 40) for n in range(3)]
        counts = rel.fragment_counts(rel.Git(runner), tags)
        self.assertEqual(
            counts, {"0.6.3.1": rel.NoteCounts(1, 0), "0.6.3.2": rel.NoteCounts(1, 0)}
        )

    def test_release_fragment_counts_invalid_path_ok(self):
        diff = "doc/operator-notes/a.md\0doc/operator-notes/sub/b.md\0"
        runner = FakeRunner({("git", "diff"): diff})
        tags = [(f"0.6.3.{n}", "d", "c" * 40) for n in range(2)]
        counts = rel.fragment_counts(rel.Git(runner), tags)
        self.assertEqual(counts, {"0.6.3.1": rel.NoteCounts(1, 1)})
        rows = rel.render_tag_log(tags, [], counts, REPO).splitlines()[2:]
        self.assertIn("| 1, \u26a0\ufe0f 1 invalid |", rows[1])


# REQ:release-operator-notes


def frag(name="a", text="### A\n\nUrgency: High\n") -> "rel.Fragment":
    return rel.Fragment(f"doc/operator-notes/{name}.md", text)


class GitFragmentsRealRepo(unittest.TestCase):
    """Runs `Git.fragments` against a scratch repository."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        cwd = os.getcwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, cwd)
        env = {
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_AUTHOR_NAME": "t",
            "GIT_AUTHOR_EMAIL": "t@example.com",
            "GIT_COMMITTER_NAME": "t",
            "GIT_COMMITTER_EMAIL": "t@example.com",
        }
        patcher = unittest.mock.patch.dict(os.environ, env)
        patcher.start()
        self.addCleanup(patcher.stop)
        self.git("init", "--quiet")
        (self.root / "doc/operator-notes").mkdir(parents=True)

    def git(self, *args):
        subprocess.run(["git", *args], check=True, capture_output=True)

    def commit(self, tag, **files):
        for name, text in files.items():
            path = self.root / "doc/operator-notes" / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        self.git("add", "-A")
        self.git("commit", "--quiet", "--allow-empty", "-m", tag)
        self.git("tag", tag)

    def test_release_git_fragments_edited_in_later_tag_ok(self):
        self.commit("0.6.0.0", **{"a.md": "### A\n\nold\n"})
        self.commit("0.6.0.1", **{"a.md": "### A\n\nnew\n"})
        self.assertEqual(
            rel.Git(remote=None).fragments("0.6.0.0", "0.6.0.1"),
            [rel.Fragment("doc/operator-notes/a.md", "### A\n\nnew\n")],
        )

    def test_release_git_fragments_delete_and_add_similar_ok(self):
        text = "### A\n\n" + "same line\n" * 20
        self.commit("0.6.0.0", **{"old.md": text})
        (self.root / "doc/operator-notes/old.md").unlink()
        self.commit("0.6.0.1", **{"new.md": text + "extra\n"})
        self.assertEqual(
            rel.Git(remote=None).fragments("0.6.0.0", "0.6.0.1"),
            [rel.Fragment("doc/operator-notes/new.md", text + "extra\n")],
        )

    def test_release_git_fragments_non_ascii_name_ok(self):
        self.commit("0.6.0.0")
        self.commit("0.6.0.1", **{"\u00fcber.md": "### U\n"})
        self.assertEqual(
            rel.Git(remote=None).fragments("0.6.0.0", "0.6.0.1"),
            [rel.Fragment("doc/operator-notes/\u00fcber.md", "### U\n")],
        )

    def test_release_git_fragments_nested_path_fails(self):
        self.commit("0.6.0.0")
        self.commit("0.6.0.1", **{"sub/a.md": "### A\n"})
        with self.assertRaisesRegex(ValueError, "doc/operator-notes/sub/a.md"):
            rel.Git(remote=None).fragments("0.6.0.0", "0.6.0.1")


class Fragments(unittest.TestCase):
    def test_release_fragment_path_ok(self):
        self.assertTrue(rel.is_fragment_path("doc/operator-notes/db-migration.md"))
        for path in (
            "doc/operator-notes/sub/a.md",
            "doc/operator-notes/notes.txt",
            "doc/other/a.md",
        ):
            self.assertFalse(rel.is_fragment_path(path), path)

    def test_release_render_fragments_sorted_ok(self):
        text = rel.render_fragments([frag("b", "### B\n"), frag("a", "### A\n\nx\n")])
        self.assertEqual(text, "### A\n\nx\n\n### B")

    def test_release_render_fragments_empty_ok(self):
        self.assertEqual(rel.render_fragments([]), rel.NO_ACTION)

    def test_release_render_fragments_bot_marker_fails(self):
        with self.assertRaisesRegex(ValueError, "doc/operator-notes/bad.md"):
            rel.render_fragments([frag("ok"), frag("bad", "### X\n<!-- publish -->")])

    def test_release_git_fragments_ok(self):
        diff = "doc/operator-notes/a.md\0doc/operator-notes/README.md\0"
        runner = FakeRunner({("git", "diff"): diff, ("git", "show"): "### A\n"})
        fragments = rel.Git(runner).fragments("0.6.0.1", "0.6.0.2")
        self.assertEqual(
            fragments, [rel.Fragment("doc/operator-notes/a.md", "### A\n")]
        )
        self.assertIn(["git", "show", "0.6.0.2:doc/operator-notes/a.md"], runner.calls)

    def test_release_git_fragments_bad_path_fails(self):
        for path in ("doc/operator-notes/sub/a.md", "doc/operator-notes/notes.txt"):
            runner = FakeRunner({("git", "diff"): f"doc/operator-notes/a.md\0{path}\0"})
            with self.assertRaisesRegex(ValueError, path):
                rel.Git(runner).fragments("0.6.0.1", "0.6.0.2")

    def test_release_git_fragment_paths_bad_path_ok(self):
        runner = FakeRunner(
            {("git", "diff"): "doc/operator-notes/a.md\0doc/operator-notes/n.txt\0"}
        )
        self.assertEqual(
            rel.Git(runner).fragment_paths("0.6.0.1", "0.6.0.2"),
            ["doc/operator-notes/a.md", "doc/operator-notes/n.txt"],
        )

    def test_release_git_fragments_readme_skipped_ok(self):
        diff = "doc/operator-notes/README.md\0doc/operator-notes/a.md\0"
        runner = FakeRunner({("git", "diff"): diff})
        self.assertEqual(
            rel.Git(runner).fragment_paths("0.6.0.1", "0.6.0.2"),
            ["doc/operator-notes/a.md"],
        )

    def test_release_tag_base_ok(self):
        tags = [t("0.6.3.0"), t("0.6.3.2"), t("0.6.3.4"), t("0.7.0.1")]
        self.assertEqual(rel.tag_base(t("0.6.3.4"), tags), t("0.6.3.2"))
        self.assertEqual(rel.tag_base(t("0.6.3.3"), tags), t("0.6.3.2"))
        self.assertEqual(rel.tag_base(t("0.7.0.1"), tags), None)
        self.assertEqual(rel.tag_base(t("0.6.3.0"), tags), None)

    def test_release_previous_line_tag_ok(self):
        tags = [t(x) for x in ("0.6.3.2", "0.6.3.14", "0.6.2.20", "0.7.0.0", "0.7.0.3")]
        self.assertEqual(rel.previous_line_tag(t("0.7.0.0"), tags), t("0.6.3.14"))
        self.assertEqual(rel.previous_line_tag(t("0.6.3.0"), tags), t("0.6.2.20"))
        self.assertIsNone(rel.previous_line_tag(t("0.6.2.0"), tags))

    def test_release_publish_base_dot_zero_uses_previous_line_ok(self):
        tags = [t("0.6.3.14"), t("0.7.0.0")]
        self.assertEqual(rel.publish_base(None, t("0.7.0.0"), tags), t("0.6.3.14"))
        self.assertEqual(
            rel.publish_base(t("0.6.3.9"), t("0.7.0.0"), tags), t("0.6.3.9")
        )
        with self.assertRaisesRegex(ValueError, "no earlier version tag"):
            rel.publish_base(None, t("0.7.0.0"), [t("0.7.0.0")])

    def test_release_git_version_tags_remote_no_fetch_ok(self):
        out = f"{'a' * 40}\trefs/tags/0.6.3.9\n{'b' * 40}\trefs/tags/0.7.0.0\n"
        runner = FakeRunner({("git", "ls-remote"): out})
        self.assertEqual(rel.Git(runner).version_tags(), [t("0.6.3.9"), t("0.7.0.0")])
        self.assertFalse(runner.ran("git", "fetch"))

    def test_release_git_version_tags_local_no_fetch_ok(self):
        runner = FakeRunner({("git", "tag"): "0.6.3.9\nnightly\n0.7.0.0\n"})
        git = rel.Git(runner, remote=None)
        self.assertEqual(git.version_tags(), [t("0.6.3.9"), t("0.7.0.0")])
        git.fetch_tag(t("0.6.3.9"))
        self.assertFalse(runner.ran("git", "fetch"))
        self.assertFalse(runner.ran("git", "ls-remote"))

    def test_release_git_fetch_tag_refspec_ok(self):
        runner = FakeRunner({("git", "fetch"): ""})
        rel.Git(runner).fetch_tag(t("0.6.3.14"))
        self.assertEqual(
            runner.calls,
            [
                [
                    "git",
                    "fetch",
                    "--quiet",
                    "origin",
                    "+refs/tags/0.6.3.14:refs/tags/0.6.3.14",
                ]
            ],
        )

    def test_release_publish_base_ok(self):
        tags = [t(f"0.7.0.{n}") for n in range(5)]
        self.assertEqual(
            rel.publish_base(t("0.7.0.1"), t("0.7.0.4"), tags), t("0.7.0.1")
        )
        self.assertEqual(
            rel.publish_base(t("0.6.3.12"), t("0.7.0.3"), tags), t("0.6.3.12")
        )
        self.assertEqual(rel.publish_base(None, t("0.7.0.3"), tags), t("0.7.0.2"))
        self.assertIsNone(rel.publish_base(t("0.7.0.4"), t("0.7.0.2"), tags))
        self.assertIsNone(rel.publish_base(t("0.7.0.1"), t("0.6.3.14"), tags))


# REQ:release-publish


GENERATED = (
    "## What's Changed\n* a in #1\n\n**Full Changelog**: https://x/compare/a...b"
)


def t(text: str) -> "rel.Tag":
    parsed = rel.parse_tag(text)
    assert parsed is not None
    return parsed


def plan(
    releases,
    target,
    network="decaf",
    notes=None,
    tags=None,
    date="2026-10-21",
    collect=None,
    whats_changed=None,
):
    tags = tags or [t(r.tag) for r in releases]
    return rel.plan_publish(
        releases,
        t(target),
        network,
        notes,
        lambda: tags,
        collect or (lambda *_: []),
        whats_changed or (lambda *_: GENERATED),
        date,
        REPO,
    )


def apply(releases, edits):
    """The releases after GitHub accepted `edits`."""
    by_tag = {edit.tag: edit for edit in edits}
    return [
        release(r.tag, by_tag[r.tag].title, by_tag[r.tag].body)
        if r.tag in by_tag
        else r
        for r in releases
    ]


def published(releases, target, network="decaf", **kwargs):
    return apply(releases, plan(releases, target, network, **kwargs)[1])


class PublishCommand(unittest.TestCase):
    def test_release_publish_parse_ok(self):
        self.assertEqual(
            rel.parse_command("/publish 0.7.0.4 decaf\r\nV0_7 at view 5"),
            rel.Command("publish", "0.7.0.4", "V0_7 at view 5", "decaf"),
        )
        self.assertEqual(
            rel.parse_command("/publish 0.7.0.4 mainnet"),
            rel.Command("publish", "0.7.0.4", None, "mainnet"),
        )

    def test_release_publish_parse_fails(self):
        for body in ("/publish", "/publish 0.7.0.4", "/publish 0.7.0.4 testnet"):
            self.assertIsNone(rel.parse_command(body), body)
        self.assertIsNone(rel.parse_command("/tag 0.7.0.4 decaf"))


class TitleNetworks(unittest.TestCase):
    def test_release_title_roundtrip_ok(self):
        for networks in ([], ["decaf"], ["mainnet"], ["decaf", "mainnet"]):
            title = rel.render_title("0.7.0.4", frozenset(networks))
            self.assertEqual(rel.title_networks(title), frozenset(networks))
        both = rel.render_title("0.7.0.4", frozenset({"mainnet", "decaf"}))
        self.assertEqual(both, "0.7.0.4 [decaf, mainnet]")
        self.assertEqual(rel.render_title("0.7.0.4", frozenset()), "0.7.0.4")

    def test_release_title_hand_written_ok(self):
        self.assertEqual(
            rel.title_networks("latest release for mainnet 0.6.3.7"), frozenset()
        )
        self.assertEqual(rel.title_networks("0.6.3.7 [other]"), frozenset())

    def test_release_current_release_ok(self):
        releases = [
            release("0.7.0.3", "0.7.0.3 [decaf]"),
            release("0.7.0.4", "0.7.0.4 [decaf]"),
            release("0.6.3.12", "0.6.3.12 [mainnet]"),
        ]
        self.assertEqual(rel.current_release(releases, "decaf"), releases[1])
        self.assertEqual(rel.current_release(releases, "mainnet"), releases[2])
        self.assertIsNone(rel.current_release(releases[:1], "mainnet"))


class FragmentRange(unittest.TestCase):
    def _plan(self, releases, target, network="decaf", tags=None):
        calls = []

        def collect(base, to):
            calls.append((str(base), str(to)))
            return [frag("a")]

        _prev, (edit, *_old) = plan(
            releases, target, network, tags=tags, collect=collect
        )
        return calls, edit

    def test_release_plan_same_line_range_ok(self):
        releases = published([release("0.7.0.1"), release("0.7.0.4")], "0.7.0.1")
        calls, edit = self._plan(releases, "0.7.0.4")
        self.assertEqual(calls, [("0.7.0.1", "0.7.0.4")])
        self.assertIn("### A\n\nUrgency: High", edit.body)

    def test_release_plan_cross_line_range_ok(self):
        releases = [
            release("0.6.3.12", "0.6.3.12 [mainnet]"),
            release("0.7.0.0"),
            release("0.7.0.3"),
        ]
        calls, _edit = self._plan(releases, "0.7.0.3", "mainnet")
        self.assertEqual(calls, [("0.6.3.12", "0.7.0.3")])

    def test_release_plan_bootstrap_uses_tag_range_ok(self):
        releases = [release("0.7.0.0"), release("0.7.0.1"), release("0.7.0.2")]
        calls, edit = self._plan(releases, "0.7.0.2")
        self.assertEqual(calls, [("0.7.0.1", "0.7.0.2")])
        self.assertIn(
            "No previous release recorded on decaf; operator actions since 0.7.0.1 only.",
            edit.body,
        )

    def test_release_plan_bootstrap_dot_zero_is_empty_ok(self):
        calls, edit = self._plan(
            [release("0.7.0.0")], "0.7.0.0", tags=[t("0.6.3.14"), t("0.7.0.0")]
        )
        self.assertEqual(calls, [("0.6.3.14", "0.7.0.0")])
        self.assertIn("operator actions since 0.6.3.14 only.", edit.body)

    def test_release_plan_rollback_collects_nothing_ok(self):
        releases = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2")
        calls, edit = self._plan(releases, "0.7.0.1")
        self.assertEqual(calls, [])
        self.assertIn("Rollback from 0.7.0.2.", edit.body)
        self.assertNotIn(rel.NO_ACTION, edit.body)

    def test_release_plan_bad_fragment_fails(self):
        releases = [release("0.7.0.0"), release("0.7.0.1")]
        with self.assertRaisesRegex(ValueError, "doc/operator-notes/bad.md"):
            plan(
                releases,
                "0.7.0.1",
                collect=lambda *_: [frag("bad", "### X\n<!-- y -->")],
            )

    def test_release_collect_fragments_fetches_base_tag_only_ok(self):
        runner = FakeRunner({("git", "fetch"): "", ("git", "diff"): ""})
        rel._collect_fragments(rel.Git(runner), t("0.6.3.12"), t("0.7.0.3"))
        fetched = [c for c in runner.calls if c[:2] == ["git", "fetch"]]
        self.assertEqual(
            [c[-1] for c in fetched], ["+refs/tags/0.6.3.12:refs/tags/0.6.3.12"]
        )


class PublishBlock(unittest.TestCase):
    def _block(self, prev, target, fragments, notes=None, base=None):
        return rel.render_publish_block(
            "decaf",
            prev and t(prev),
            t(target),
            "2026-10-21",
            notes,
            fragments,
            REPO,
            base and t(base),
        )

    def test_release_publish_block_header_ok(self):
        block = self._block("0.7.0.1", "0.7.0.3", [])
        self.assertTrue(
            block.startswith("<!-- publish:decaf prev=0.7.0.1 date=2026-10-21 -->\n")
        )
        self.assertTrue(block.endswith("<!-- /publish:decaf -->"))
        self.assertIn(
            "> Deployable on **decaf** since 2026-10-21. Supersedes 0.7.0.1.", block
        )
        self.assertIn(
            "> `docker pull ghcr.io/espressosystems/espresso-network/espresso-node:0.7.0.3`",
            block,
        )
        self.assertIn("## Operator actions for decaf since 0.7.0.1", block)
        self.assertIn(rel.NO_ACTION, block)
        self.assertIn(
            f"Compare: [0.7.0.1...0.7.0.3](https://github.com/{REPO}/compare/0.7.0.1...0.7.0.3)",
            block,
        )

    def test_release_publish_block_fragments_ok(self):
        block = self._block(
            "0.7.0.1", "0.7.0.4", [frag("b", "### B\n"), frag("a")], "V0_7 at view 5"
        )
        self.assertIn("### A\n\nUrgency: High\n\n### B", block)
        self.assertNotIn(rel.NO_ACTION, block)
        self.assertLess(block.index("V0_7 at view 5"), block.index("### A"))
        self.assertEqual(rel.publish_block_notes(block, "decaf"), "V0_7 at view 5")

    def test_release_publish_block_bootstrap_ok(self):
        block = self._block(None, "0.6.3.12", [frag()])
        self.assertIn("prev=none", block)
        self.assertIn("## Operator actions for decaf\n", block)
        self.assertNotIn("Supersedes", block)
        self.assertNotIn("Compare", block)

    def test_release_publish_block_first_publish_says_so_ok(self):
        block = self._block(None, "0.6.3.12", [frag()], base="0.6.3.11")
        message = "No previous release recorded on decaf; operator actions since 0.6.3.11 only."
        self.assertIn(message, block)
        self.assertLess(block.index(message), block.index("### A"))

    def test_release_publish_block_first_publish_empty_says_so_ok(self):
        block = self._block(None, "0.6.3.12", [], base="0.6.3.11")
        self.assertIn("No previous release recorded on decaf;", block)
        self.assertIn(rel.NO_ACTION, block)

    def test_release_publish_block_first_dot_zero_says_so_ok(self):
        block = self._block(None, "0.6.3.0", [])
        self.assertIn("No previous release recorded on decaf.", block)

    def test_release_publish_block_with_prev_has_no_first_message_ok(self):
        block = self._block("0.7.0.1", "0.7.0.3", [], base="0.7.0.1")
        self.assertNotIn("No previous release", block)

    def test_release_publish_block_rollback_ok(self):
        block = self._block("0.7.0.4", "0.7.0.2", [])
        self.assertIn("Rollback from 0.7.0.4.", block)
        self.assertNotIn("Operator actions", block)
        self.assertNotIn("Compare", block)

    def test_release_publish_block_prev_ok(self):
        block = self._block("0.7.0.1", "0.7.0.3", [])
        self.assertEqual(
            rel.publish_block_prev(block, "decaf"), ("0.7.0.1", "2026-10-21")
        )
        self.assertIsNone(rel.publish_block_prev(block, "mainnet"))
        none = self._block(None, "0.7.0.3", [])
        self.assertEqual(rel.publish_block_prev(none, "decaf"), (None, "2026-10-21"))

    def test_release_with_block_keeps_hand_text_ok(self):
        first = self._block(None, "0.7.0.3", [])
        body = rel.with_block("Hand written summary.", "decaf", first)
        self.assertTrue(body.startswith("<!-- publish:decaf"))
        self.assertTrue(body.endswith("\n\nHand written summary."))
        second = self._block("0.7.0.1", "0.7.0.3", [])
        again = rel.with_block(body, "decaf", second)
        self.assertEqual(again.count("<!-- publish:decaf"), 1)
        self.assertIn("prev=0.7.0.1", again)
        self.assertTrue(again.endswith("Hand written summary."))

    def test_release_mark_superseded_ok(self):
        block = self._block(None, "0.7.0.3", [])
        body = rel.with_block("Hand written.", "decaf", block)
        once = rel.mark_superseded(body, "decaf", "0.7.0.4", "2026-10-23")
        self.assertIn(
            "<!-- superseded:decaf -->\nSuperseded on decaf by 0.7.0.4 (2026-10-23)\n"
            "<!-- /superseded:decaf -->\n<!-- /publish:decaf -->",
            once,
        )
        twice = rel.mark_superseded(once, "decaf", "0.7.0.5", "2026-10-24")
        self.assertEqual(twice.count("Superseded on"), 1)
        self.assertIn("by 0.7.0.5 (2026-10-24)", twice)
        self.assertTrue(twice.endswith("Hand written."))
        self.assertEqual(rel.publish_block_prev(twice, "decaf"), (None, "2026-10-21"))

    def test_release_mark_superseded_keeps_notes_lines_ok(self):
        block = rel.render_publish_block(
            "decaf",
            None,
            t("0.7.0.3"),
            "2026-10-21",
            "Superseded on decaf by the ops team",
            [],
            REPO,
            None,
        )
        once = rel.mark_superseded(block, "decaf", "0.7.0.4", "2026-10-23")
        twice = rel.mark_superseded(once, "decaf", "0.7.0.5", "2026-10-24")
        self.assertIn("Superseded on decaf by the ops team", twice)
        self.assertIn("<!-- superseded:decaf -->", twice)
        self.assertEqual(twice.count("by 0.7.0.5 (2026-10-24)"), 1)
        self.assertNotIn("0.7.0.4", twice)

    def test_release_mark_superseded_without_block_ok(self):
        self.assertEqual(rel.mark_superseded("plain", "decaf", "0.7.0.4", "d"), "plain")


class PlanPublish(unittest.TestCase):
    def test_release_plan_bootstrap_ok(self):
        releases = [release("0.7.0.0"), release("0.7.0.1")]
        prev, edits = plan(releases, "0.7.0.1", collect=lambda *_: [frag()])
        self.assertIsNone(prev)
        (edit,) = edits
        self.assertEqual(edit.title, "0.7.0.1 [decaf]")
        self.assertFalse(edit.make_latest)
        self.assertIn("prev=none", edit.body)
        self.assertIn("### A\n\nUrgency: High", edit.body)

    def test_release_plan_supersedes_previous_ok(self):
        releases = published(
            [
                release("0.7.0.1"),
                release("0.7.0.2"),
                release("0.7.0.3"),
            ],
            "0.7.0.1",
        )
        prev, edits = plan(releases, "0.7.0.3", date="2026-10-23")
        self.assertEqual(prev, t("0.7.0.1"))
        new, old = edits
        self.assertEqual(
            (old.tag, old.title, old.make_latest),
            ("0.7.0.1", "0.7.0.1", False),
        )
        self.assertIn("Superseded on decaf by 0.7.0.3 (2026-10-23)", old.body)
        self.assertEqual(new.title, "0.7.0.3 [decaf]")
        self.assertIn("Supersedes 0.7.0.1", new.body)
        self.assertIn(rel.NO_ACTION, new.body)

    def test_release_plan_mainnet_takes_latest_ok(self):
        releases = [
            release("0.6.3.12", "0.6.3.12 [decaf, mainnet]"),
            release("0.7.0.0"),
            release("0.7.0.1"),
        ]
        prev, edits = plan(
            releases, "0.7.0.1", "mainnet", tags=[t("0.7.0.0"), t("0.7.0.1")]
        )
        self.assertEqual(prev, t("0.6.3.12"))
        new, old = edits
        self.assertEqual((old.title, old.make_latest), ("0.6.3.12 [decaf]", False))
        self.assertEqual((new.title, new.make_latest), ("0.7.0.1 [mainnet]", True))
        self.assertIn("## Operator actions for mainnet since 0.6.3.12", new.body)
        self.assertIn(rel.NO_ACTION, new.body)

    def test_release_plan_old_holder_keeps_latest_on_other_network_ok(self):
        releases = [
            release("0.6.3.12", "0.6.3.12 [decaf, mainnet]"),
            release("0.6.3.13"),
        ]
        _prev, edits = plan(releases, "0.6.3.13", "decaf", tags=[t("0.6.3.13")])
        new, old = edits
        self.assertEqual((old.title, old.make_latest), ("0.6.3.12 [mainnet]", True))
        self.assertEqual((new.title, new.make_latest), ("0.6.3.13 [decaf]", False))

    def test_release_plan_none_clears_network_notes_ok(self):
        releases = published(
            [release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2", notes="Deadline view 5"
        )
        _prev, (cleared,) = plan(releases, "0.7.0.2", notes="none")
        self.assertNotIn("Deadline view 5", cleared.body)
        self.assertNotIn("network-notes", cleared.body)
        self.assertIsNone(rel.publish_block_notes(cleared.body, "decaf"))
        _prev, (fresh,) = plan([release("0.7.0.1")], "0.7.0.1", notes="none")
        self.assertNotIn("network-notes", fresh.body)

    def test_release_plan_missing_release_creates_ok(self):
        prev, (create,) = plan([release("0.7.0.0")], "0.7.0.5")
        self.assertIsNone(prev)
        self.assertEqual(create.action, "create")
        self.assertEqual(create.title, "0.7.0.5 [decaf]")
        self.assertTrue(create.body.startswith("<!-- publish:decaf prev=none"))
        self.assertIn(f"{rel.WHATS_CHANGED_OPEN}\n{GENERATED}\n", create.body)

    def test_release_plan_whats_changed_decaf_uses_decaf_prev_ok(self):
        calls = []
        releases = published([release("0.7.0.1")], "0.7.0.1")
        plan(
            releases + [release("0.7.0.3")],
            "0.7.0.3",
            tags=[t("0.7.0.2")],
            whats_changed=lambda *a: calls.append(a) or GENERATED,
        )
        self.assertEqual(calls, [(t("0.7.0.3"), t("0.7.0.1"))])

    def test_release_plan_whats_changed_mainnet_prev_wins_ok(self):
        calls = []
        releases = [
            release("0.6.3.12", "0.6.3.12 [decaf, mainnet]"),
            release("0.6.3.14", "0.6.3.14 [decaf]"),
            release("0.6.3.15"),
        ]
        releases = published(releases, "0.6.3.15", tags=[t("0.6.3.14")])
        releases = published(releases, "0.6.3.15", "mainnet", tags=[t("0.6.3.14")])
        for network in ("decaf", "mainnet"):
            plan(
                releases,
                "0.6.3.15",
                network,
                whats_changed=lambda *a: calls.append(a) or GENERATED,
            )
        self.assertEqual([c[1] for c in calls], [t("0.6.3.12")] * 2)

    def test_release_plan_whats_changed_mainnet_without_prev_uses_previous_tag_ok(self):
        calls = []
        releases = published([release("0.7.0.1")], "0.7.0.1")
        releases = published(releases + [release("0.7.0.3")], "0.7.0.3")
        plan(
            releases,
            "0.7.0.3",
            "mainnet",
            tags=[t("0.7.0.1"), t("0.7.0.2"), t("0.7.0.3")],
            whats_changed=lambda *a: calls.append(a) or GENERATED,
        )
        self.assertEqual(calls, [(t("0.7.0.3"), t("0.7.0.2"))])

    def test_release_plan_whats_changed_no_prev_uses_previous_tag_ok(self):
        calls = []
        plan(
            [release("0.7.0.0")],
            "0.7.0.5",
            whats_changed=lambda *a: calls.append(a) or GENERATED,
        )
        self.assertEqual(calls, [(t("0.7.0.5"), t("0.7.0.0"))])

    def test_release_plan_whats_changed_rollback_untouched_ok(self):
        calls = []
        generate = lambda *a: calls.append(a) or GENERATED
        marked = f"{rel.WHATS_CHANGED_OPEN}\nOLD\n{rel.WHATS_CHANGED_CLOSE}"
        releases = published(
            [release("0.7.0.1", body=marked), release("0.7.0.3")], "0.7.0.3"
        )
        _prev, (edit, *_) = plan(releases, "0.7.0.1", whats_changed=generate)
        self.assertEqual(calls, [])
        self.assertIn(marked, edit.body)

    def test_release_plan_whats_changed_create_rollback_has_no_section_ok(self):
        releases = published([release("0.7.0.3")], "0.7.0.3")
        _prev, (create, _old) = plan(releases, "0.7.0.1")
        self.assertEqual(create.action, "create")
        self.assertNotIn(rel.WHATS_CHANGED_OPEN, create.body)

    def test_release_plan_whats_changed_rerun_idempotent_ok(self):
        releases = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2")
        _prev, (again,) = plan(releases, "0.7.0.2")
        self.assertEqual(again.body, releases[1].body)

    def test_release_plan_create_mainnet_takes_latest_ok(self):
        _prev, (create,) = plan([release("0.7.0.0")], "0.7.0.5", "mainnet")
        self.assertTrue(create.make_latest)

    def test_release_plan_existing_release_is_edited_ok(self):
        _prev, (edit,) = plan([release("0.7.0.1")], "0.7.0.1")
        self.assertEqual(edit.action, "edit")

    def test_release_plan_mainnet_promotion_recreates_ok(self):
        releases = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2")
        _prev, (target,) = plan(releases, "0.7.0.2", "mainnet")
        self.assertEqual(target.action, "recreate")
        self.assertEqual(target.old, releases[1])

    def test_release_plan_mainnet_rollback_recreates_ok(self):
        releases = published(
            [release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2", "mainnet"
        )
        _prev, (target, superseded) = plan(releases, "0.7.0.1", "mainnet")
        self.assertEqual(target.action, "recreate")
        self.assertEqual(superseded.action, "edit")

    def test_release_plan_rerun_after_delete_before_create_creates_ok(self):
        releases = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2")
        deleted = [releases[0]]
        _prev, (target,) = plan(
            deleted, "0.7.0.2", "mainnet", tags=[t("0.7.0.1"), t("0.7.0.2")]
        )
        self.assertEqual(target.action, "create")
        self.assertEqual(target.title, "0.7.0.2 [mainnet]")
        self.assertNotIn("publish:decaf", target.body)

    def test_release_plan_rerun_after_recreate_before_supersede_edits_ok(self):
        releases = published(
            [release("0.7.0.1"), release("0.7.0.2")], "0.7.0.1", "mainnet"
        )
        releases = published(releases, "0.7.0.2", "decaf")
        _prev, edits = plan(releases, "0.7.0.2", "mainnet")
        self.assertEqual(edits[0].action, "recreate")
        partial = apply(releases, edits[:1])
        _prev, again = plan(partial, "0.7.0.2", "mainnet")
        self.assertEqual(
            [(e.tag, e.action) for e in again],
            [("0.7.0.2", "edit"), ("0.7.0.1", "edit")],
        )
        self.assertNotIn("mainnet", rel.title_networks(again[1].title))

    def test_release_plan_mainnet_rerun_of_holder_edits_ok(self):
        releases = published([release("0.7.0.1")], "0.7.0.1", "mainnet")
        _prev, (target,) = plan(releases, "0.7.0.1", "mainnet")
        self.assertEqual(target.action, "edit")

    def test_release_plan_decaf_existing_release_edits_ok(self):
        releases = published([release("0.7.0.1")], "0.7.0.1", "mainnet")
        _prev, (target,) = plan(releases, "0.7.0.1", "decaf")
        self.assertEqual(target.action, "edit")

    def test_release_plan_rerun_keeps_prev_date_and_notes_ok(self):
        releases = published(
            [release("0.7.0.1"), release("0.7.0.2"), release("0.7.0.3")],
            "0.7.0.1",
        )
        releases = published(releases, "0.7.0.2", notes="Deadline view 5")
        prev, (again,) = plan(releases, "0.7.0.2", date="2026-10-30")
        self.assertEqual(prev, t("0.7.0.1"))
        self.assertIn("date=2026-10-21", again.body)
        self.assertIn("Deadline view 5", again.body)
        self.assertEqual(again.body.count("<!-- publish:decaf"), 1)
        _prev, (replaced,) = plan(releases, "0.7.0.2", notes="Deadline view 9")
        self.assertIn("Deadline view 9", replaced.body)
        self.assertNotIn("Deadline view 5", replaced.body)

    def test_release_plan_rerun_supersedes_leftover_holder_ok(self):
        releases = [
            release("0.7.0.1", "0.7.0.1 [decaf]"),
            release("0.7.0.2", title="0.7.0.2 [decaf]"),
        ]
        _prev, edits = plan(releases, "0.7.0.2")
        self.assertEqual([e.tag for e in edits], ["0.7.0.2", "0.7.0.1"])

    def test_release_plan_rerun_after_first_edit_only_ok(self):
        releases = published(
            [
                release("0.7.0.1"),
                release("0.7.0.2"),
                release("0.7.0.3"),
            ],
            "0.7.0.1",
        )
        prev, edits = plan(releases, "0.7.0.3", notes="Deadline view 5")
        self.assertEqual(len(edits), 2)
        partial = apply(releases, edits[:1])
        again_prev, again = plan(partial, "0.7.0.3")
        self.assertEqual(again_prev, prev)
        self.assertEqual(again, edits)

    def test_release_plan_lost_holder_title_fails(self):
        releases = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.1")
        releases[0] = release("0.7.0.1", "renamed by hand", releases[0].body)
        with self.assertRaises(ValueError) as caught:
            plan(releases, "0.7.0.2")
        self.assertIn("0.7.0.1", str(caught.exception))
        self.assertIn("title", str(caught.exception))

    def test_release_plan_superseded_block_is_not_lost_ok(self):
        releases = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.1")
        releases = published(releases, "0.7.0.2")
        releases[1] = release("0.7.0.2")
        prev, _edits = plan(releases, "0.7.0.1")
        self.assertIsNone(prev)

    def test_release_plan_rollback_ok(self):
        releases = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2")
        prev, edits = plan(releases, "0.7.0.1")
        self.assertEqual(prev, t("0.7.0.2"))
        self.assertIn("Rollback from 0.7.0.2.", edits[0].body)
        self.assertEqual(edits[1].title, "0.7.0.2")

    def test_release_plan_republish_drops_stale_block_ok(self):
        releases = published(
            [release("0.7.0.1"), release("0.7.0.2")], "0.7.0.1", notes="Old deadline"
        )
        releases = published(releases, "0.7.0.2")
        self.assertIn("Superseded", releases[0].body)
        _prev, edits = plan(releases, "0.7.0.1")
        self.assertNotIn("Old deadline", edits[0].body)
        self.assertNotIn("Superseded", edits[0].body)
        self.assertEqual(edits[0].body.count("<!-- publish:decaf"), 1)


class DeployedLine(unittest.TestCase):
    def test_release_deployed_line_ok(self):
        releases = [
            release("0.7.0.4", "0.7.0.4 [decaf]"),
            release("0.6.3.12", "0.6.3.12 [mainnet]"),
        ]
        line = rel.render_deployed_line(releases, REPO)
        self.assertEqual(
            line,
            f"Deployed: mainnet [0.6.3.12](https://github.com/{REPO}/releases/tag/0.6.3.12)"
            f" · decaf [0.7.0.4](https://github.com/{REPO}/releases/tag/0.7.0.4)",
        )
        self.assertEqual(
            rel.render_deployed_line([], REPO), "Deployed: mainnet - · decaf -"
        )

    def test_release_body_starts_with_deployed_line_ok(self):
        body = rel.render_body(state(), "", REPO)
        self.assertTrue(body.startswith("Deployed: "))

    def test_release_release_cell_networks_ok(self):
        releases = [
            release("0.7.0.4", "0.7.0.4 [decaf, mainnet]"),
            release("0.7.0.5"),
        ]
        cell = rel.render_release_cell("0.7.0.4", releases, REPO)
        self.assertTrue(cell.startswith("[release]("))
        self.assertTrue(cell.endswith(" · decaf, mainnet"))
        self.assertTrue(
            rel.render_release_cell("0.7.0.5", releases, REPO).endswith(")")
        )


class PublishComment(unittest.TestCase):
    def test_release_publish_comment_ok(self):
        comment = rel.render_publish_comment(
            t("0.7.0.3"), "decaf", t("0.7.0.1"), REPO, "alice", "https://run"
        )
        self.assertIn(
            "Published `0.7.0.3` on `decaf`, superseding `0.7.0.1`, requested by @alice.",
            comment,
        )
        self.assertIn(
            "docker pull ghcr.io/espressosystems/espresso-network/espresso-node:0.7.0.3",
            comment,
        )
        self.assertIn("[Workflow run](https://run)", comment)

    def test_release_publish_comment_rollback_ok(self):
        comment = rel.render_publish_comment(
            t("0.7.0.1"), "decaf", t("0.7.0.3"), REPO, None, None
        )
        self.assertIn("Rolled `decaf` back to `0.7.0.1` from `0.7.0.3`.", comment)


class CmdPublish(unittest.TestCase):
    def _runner(self, status="200", releases=None, perm="write"):
        rows = releases or [
            release_row("0.7.0.1"),
            release_row("0.7.0.2"),
        ]
        return FakeRunner(
            {
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.7.0", "body": ""}
                ),
                ("gh", "issue", "list"): json.dumps(
                    [{"number": 42, "title": "Release 0.7.0"}]
                ),
                ("gh", "repo", "view"): REPO,
                ("gh", "api", "repos/{owner}/{repo}/releases?per_page=100"): "\n".join(
                    rows
                )
                + "\n",
                (
                    "gh",
                    "api",
                    "repos/{owner}/{repo}/releases/generate-notes",
                ): json.dumps({"name": "n", "body": GENERATED}),
                ("gh", "release", "edit"): "",
                ("curl", "-fsS"): '{"token": "anon"}',
                ("curl", "-sS"): status,
                ("gh", "workflow", "run"): "",
                ("gh", "issue", "comment"): "",
                ("gh", "issue", "edit"): "",
                ("gh", "issue", "create"): "https://example/issues/9\n",
                ("gh", "label", "create"): "",
                ("git", "fetch"): "",
                ("git", "diff"): "",
                ("git", "show"): "### A\n",
                (
                    "git",
                    "tag",
                    "--list",
                    "0.7.0.*",
                ): "0.7.0.0\n0.7.0.1\n0.7.0.2\n0.7.0.3\n",
                PERMISSION: perm,
            }
        )

    def _args(
        self, comment="/publish 0.7.0.2 decaf", issue=42, actor="alice", dry_run=False
    ):
        return argparse.Namespace(
            issue=issue, comment=comment, actor=actor, run_url=None, dry_run=dry_run
        )

    def test_release_cmd_publish_ok(self):
        runner = self._runner()
        code = rel.cmd_publish(
            self._args("/publish 0.7.0.2 decaf\nDeadline view 5"),
            rel.Git(runner),
            rel.Gh(runner),
        )
        self.assertEqual(code, 0, runner.calls)
        edit_call = next(c for c in runner.calls if c[:3] == ["gh", "release", "edit"])
        self.assertEqual(edit_call[3], "0.7.0.2")
        self.assertEqual(edit_call[edit_call.index("--title") + 1], "0.7.0.2 [decaf]")
        self.assertIn("--prerelease=false", edit_call)
        self.assertIn("--latest=false", edit_call)
        self.assertTrue(
            runner.ran("gh", "workflow", "run", "update-release-tracker.yml")
        )
        comments = [c for c in runner.calls if c[:3] == ["gh", "issue", "comment"]]
        self.assertEqual([c[3] for c in comments], ["42"])
        self.assertIn("Published `0.7.0.2` on `decaf`", comments[0][-1])
        self.assertFalse(runner.ran("gh", "issue", "create"))

    def test_release_cmd_publish_creates_missing_release_ok(self):
        runner = self._runner(releases=[release_row("0.7.0.1")])
        runner.responses[("gh", "release", "create")] = ""
        code = rel.cmd_publish(
            self._args("/publish 0.7.0.2 decaf"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 0, runner.calls)
        create = next(c for c in runner.calls if c[:3] == ["gh", "release", "create"])
        self.assertEqual(create[3], "0.7.0.2")
        self.assertIn("--verify-tag", create)
        self.assertNotIn("--generate-notes", create)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comments = [c for c in runner.calls if c[:3] == ["gh", "issue", "comment"]]
        self.assertIn("Published `0.7.0.2` on `decaf`", comments[0][-1])

    def test_release_cmd_publish_mainnet_promotion_recreates_ok(self):
        runner = self._runner()
        runner.responses[("gh", "release", "delete")] = ""
        runner.responses[("gh", "release", "create")] = ""
        code = rel.cmd_publish(
            self._args("/publish 0.7.0.2 mainnet"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 0, runner.calls)
        delete = next(c for c in runner.calls if c[:3] == ["gh", "release", "delete"])
        self.assertEqual(delete[3], "0.7.0.2")
        self.assertNotIn("--cleanup-tag", delete)
        create = next(c for c in runner.calls if c[:3] == ["gh", "release", "create"])
        self.assertIn("--latest=true", create)
        self.assertLess(runner.calls.index(delete), runner.calls.index(create))
        self.assertFalse(runner.ran("gh", "release", "edit", "0.7.0.2"))

    def test_release_cmd_publish_mainnet_recreate_prints_old_body_ok(self):
        rows = [release_row("0.7.0.1"), release_row("0.7.0.2", body="old decaf block")]
        runner = self._runner(releases=rows)
        runner.responses[("gh", "release", "delete")] = ""
        runner.responses[("gh", "release", "create")] = ""
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rel.cmd_publish(
                self._args("/publish 0.7.0.2 mainnet"), rel.Git(runner), rel.Gh(runner)
            )
        self.assertEqual(code, 0, runner.calls)
        self.assertIn("old decaf block", out.getvalue())

    def test_release_cmd_publish_mainnet_rollback_orders_calls_ok(self):
        held = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2", "mainnet")
        rows = [release_row(r.tag, r.title, r.body) for r in held]
        runner = self._runner(releases=rows)
        runner.responses[("gh", "release", "delete")] = ""
        runner.responses[("gh", "release", "create")] = ""
        code = rel.cmd_publish(
            self._args("/publish 0.7.0.1 mainnet"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 0, runner.calls)
        create = next(c for c in runner.calls if c[:3] == ["gh", "release", "create"])
        edit = next(c for c in runner.calls if c[:3] == ["gh", "release", "edit"])
        self.assertIn("--latest=true", create)
        self.assertEqual(edit[3], "0.7.0.2")
        self.assertIn("--latest=false", edit)
        self.assertLess(runner.calls.index(create), runner.calls.index(edit))

    def test_release_cmd_publish_mainnet_rerun_edits_ok(self):
        held = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.2", "mainnet")
        rows = [release_row(r.tag, r.title, r.body) for r in held]
        runner = self._runner(releases=rows)
        code = rel.cmd_publish(
            self._args("/publish 0.7.0.2 mainnet"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 0, runner.calls)
        edit_call = next(c for c in runner.calls if c[:3] == ["gh", "release", "edit"])
        self.assertIn("--latest=true", edit_call)
        self.assertFalse(runner.ran("gh", "release", "delete"))
        self.assertFalse(runner.ran("gh", "release", "create"))

    def test_release_cmd_publish_bot_marker_fails(self):
        runner = self._runner()
        code = rel.cmd_publish(
            self._args("/publish 0.7.0.2 decaf\n<!-- /publish:decaf -->"),
            rel.Git(runner),
            rel.Gh(runner),
        )
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("/publish failed:", comment[-1])
        self.assertIn("must not contain `<!--`", comment[-1])
        self.assertNotIn("<!-- /publish", comment[-1])

    def test_release_cmd_publish_lost_holder_title_comments_fails(self):
        held = published([release("0.7.0.1"), release("0.7.0.2")], "0.7.0.1")[0]
        rows = [
            release_row("0.7.0.1", "renamed by hand", held.body),
            release_row("0.7.0.2"),
        ]
        runner = self._runner(releases=rows)
        code = rel.cmd_publish(self._args(), rel.Git(runner), rel.Gh(runner))
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("/publish failed:", comment[-1])
        self.assertIn("0.7.0.1", comment[-1])

    def test_release_cmd_publish_build_gate_fails(self):
        runner = self._runner(status="404")
        code = rel.cmd_publish(self._args(), rel.Git(runner), rel.Gh(runner))
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn(
            "/publish failed: image "
            "ghcr.io/espressosystems/espresso-network/espresso-node:0.7.0.2 not found",
            comment[-1],
        )

    def _failing_gate_runner(self):
        runner = self._runner()

        def run(argv):
            if argv[:2] == ["curl", "-sS"]:
                raise RuntimeError("curl exited 6: could not resolve host")
            return runner(argv)

        return run, runner

    def test_release_cmd_publish_gate_error_reported_fails(self):
        run, runner = self._failing_gate_runner()
        self.assertEqual(rel.cmd_publish(self._args(), rel.Git(run), rel.Gh(run)), 1)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("/publish failed:", comment[-1])
        self.assertIn("could not resolve host", comment[-1])

    def test_release_cmd_publish_unreadable_token_reported_fails(self):
        runner = self._runner()
        runner.responses[("curl", "-fsS")] = "<html>"
        self.assertEqual(
            rel.cmd_publish(self._args(), rel.Git(runner), rel.Gh(runner)), 1
        )
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("/publish failed:", comment[-1])
        self.assertIn(
            "ghcr.io token response for espressosystems/espresso-network/espresso-node unreadable",
            comment[-1],
        )

    def test_release_cmd_publish_gate_error_dry_run_no_comment_fails(self):
        run, runner = self._failing_gate_runner()
        args = self._args(dry_run=True)
        self.assertEqual(rel.cmd_publish(args, rel.Git(run), rel.Gh(run)), 1)
        self.assertFalse(runner.ran("gh", "issue", "comment"))

    def test_release_run_error_omits_auth_header_fails(self):
        argv = ["sh", "-c", "exit 3", "x", "-H", "Authorization: Bearer s3cret", "y"]
        with self.assertRaises(RuntimeError) as caught:
            rel._run(argv)
        self.assertNotIn("s3cret", str(caught.exception))
        self.assertIsNone(caught.exception.__cause__)
        self.assertTrue(caught.exception.__suppress_context__)
        self.assertIn("exited 3", str(caught.exception))

    def test_release_cmd_publish_fragments_in_block_ok(self):
        runner = self._runner()
        runner.responses[("git", "diff")] = (
            "doc/operator-notes/a.md\0doc/operator-notes/README.md\0"
        )
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rel.cmd_publish(
                self._args(issue=None, actor=None, dry_run=True),
                rel.Git(runner),
                rel.Gh(runner),
            )
        self.assertEqual(code, 0)
        self.assertIn(
            [
                "git",
                "diff",
                "--no-renames",
                "--diff-filter=AM",
                "--name-only",
                "-z",
                "0.7.0.1",
                "0.7.0.2",
                "--",
                "doc/operator-notes/",
            ],
            runner.calls,
        )
        self.assertIn("### A", out.getvalue())

    def test_release_cmd_publish_cross_line_diffs_previous_holder_ok(self):
        rows = [
            release_row("0.6.3.12", "0.6.3.12 [mainnet]"),
            release_row("0.7.1.1"),
        ]
        runner = self._runner(releases=rows)
        runner.responses[("git", "tag", "--list", "0.7.1.*")] = "0.7.1.0\n0.7.1.1\n"
        runner.responses[("gh", "release", "delete")] = ""
        runner.responses[("gh", "release", "create")] = ""
        runner.responses[("gh", "issue", "view")] = json.dumps(
            {"title": "Release 0.7.1", "body": ""}
        )
        args = self._args("/publish 0.7.1.1 mainnet")
        self.assertEqual(rel.cmd_publish(args, rel.Git(runner), rel.Gh(runner)), 0)
        diff = next(c for c in runner.calls if c[:2] == ["git", "diff"])
        self.assertEqual(diff[6:8], ["0.6.3.12", "0.7.1.1"])

    def test_release_cmd_publish_first_dot_zero_lists_fragments_ok(self):
        runner = self._runner(releases=[release_row("0.7.0.0")])
        runner.responses[("git", "ls-remote", "--tags")] = ls_remote(
            "0.6.3.9", "0.7.0.0"
        )
        runner.responses[("git", "diff")] = "doc/operator-notes/a.md\0"
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rel.cmd_publish(
                self._args(
                    "/publish 0.7.0.0 decaf", issue=None, actor=None, dry_run=True
                ),
                rel.Git(runner),
                rel.Gh(runner),
            )
        self.assertEqual(code, 0)
        diff = next(c for c in runner.calls if c[:2] == ["git", "diff"])
        self.assertEqual(diff[6:8], ["0.6.3.9", "0.7.0.0"])
        self.assertIn("operator actions since 0.6.3.9 only.", out.getvalue())
        self.assertIn("### A", out.getvalue())

    def test_release_cmd_publish_dot_zero_no_ancestor_fails(self):
        runner = self._runner(releases=[release_row("0.7.0.0")])
        runner.responses[("git", "ls-remote", "--tags")] = ls_remote("0.7.0.0")
        code = rel.cmd_publish(
            self._args("/publish 0.7.0.0 decaf"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("no earlier version tag", comment[-1])

    def test_release_cmd_publish_bad_fragment_fails(self):
        runner = self._runner()
        runner.responses[("git", "diff")] = "doc/operator-notes/bad.md\0"
        runner.responses[("git", "show")] = "### X\n<!-- /publish:decaf -->"
        code = rel.cmd_publish(self._args(), rel.Git(runner), rel.Gh(runner))
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("doc/operator-notes/bad.md", comment[-1])

    def test_release_cmd_publish_wrong_version_fails(self):
        runner = self._runner()
        code = rel.cmd_publish(
            self._args("/publish 0.6.3.1 decaf"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("gh", "release", "edit"))
        comment = next(c for c in runner.calls if c[:3] == ["gh", "issue", "comment"])
        self.assertIn("does not belong to 0.7.0", comment[-1])

    def test_release_cmd_publish_non_writer_fails(self):
        runner = FakeRunner(
            {
                (
                    "gh",
                    "api",
                    "repos/{owner}/{repo}/collaborators/mallory/permission",
                ): "read"
            }
        )
        code = rel.cmd_publish(
            self._args(actor="mallory"), rel.Git(runner), rel.Gh(runner)
        )
        self.assertEqual(code, 1)
        self.assertEqual(len(runner.calls), 1)

    def test_release_cmd_publish_dry_run_writes_nothing_ok(self):
        runner = self._runner()
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rel.cmd_publish(
                self._args(
                    "/publish 0.7.0.2 decaf", issue=None, actor=None, dry_run=True
                ),
                rel.Git(runner),
                rel.Gh(runner),
            )
        self.assertEqual(code, 0)
        self.assertIn(
            "# 0.7.0.2: 0.7.0.2 [decaf] (edit, make_latest=False)",
            out.getvalue(),
        )
        self.assertIn("docker pull", out.getvalue())
        for write in (
            ("gh", "release", "edit"),
            ("gh", "release", "create"),
            ("gh", "issue", "comment"),
            ("gh", "issue", "create"),
            ("gh", "issue", "edit"),
            ("gh", "workflow"),
        ):
            self.assertFalse(runner.ran(*write))


class ImageExists(unittest.TestCase):
    def _gh(self, status: str):
        runner = FakeRunner(
            {("curl", "-fsS"): '{"token": "anon"}', ("curl", "-sS"): status}
        )
        return runner, rel.Gh(runner)

    def test_release_image_exists_ok(self):
        runner, gh = self._gh("200")
        self.assertTrue(gh.image_exists("EspressoSystems/espresso-network", "0.7.0.3"))
        self.assertIn(
            "https://ghcr.io/token?scope=repository:espressosystems/espresso-network/espresso-node:pull",
            runner.calls[0],
        )
        self.assertIn(
            "https://ghcr.io/v2/espressosystems/espresso-network/espresso-node/manifests/0.7.0.3",
            runner.calls[1],
        )
        self.assertIn("Authorization: Bearer anon", runner.calls[1])

    def test_release_image_missing_ok(self):
        _runner, gh = self._gh("404")
        self.assertFalse(gh.image_exists(REPO, "9.9.9.9"))

    def test_release_image_unexpected_status_fails(self):
        _runner, gh = self._gh("500")
        with self.assertRaisesRegex(RuntimeError, "returned 500"):
            gh.image_exists(REPO, "0.7.0.3")


class GhReleaseEdit(unittest.TestCase):
    def test_release_gh_release_edit_sends_latest_ok(self):
        runner = FakeRunner({("gh", "release", "edit"): ""})
        edit = rel.ReleaseEdit("0.7.0.1", "0.7.0.1 [decaf]", "body")
        rel.Gh(runner).release_edit(edit)
        call = runner.calls[-1]
        self.assertEqual(call[:4], ["gh", "release", "edit", "0.7.0.1"])
        self.assertIn("--prerelease=false", call)
        self.assertIn("--latest=false", call)
        self.assertIn("--notes-file", call)

    def test_release_gh_release_edit_mainnet_latest_ok(self):
        runner = FakeRunner({("gh", "release", "edit"): ""})
        edit = rel.ReleaseEdit("0.7.0.1", "0.7.0.1 [mainnet]", "body")
        rel.Gh(runner).release_edit(edit)
        self.assertIn("--latest=true", runner.calls[-1])


class GhReleaseCreate(unittest.TestCase):
    def test_release_gh_release_create_flags_ok(self):
        runner = FakeRunner({("gh", "release", "create"): ""})
        edit = rel.ReleaseEdit("0.7.0.1", "0.7.0.1 [mainnet]", "body", "create")
        rel.Gh(runner).release_create(edit)
        call = runner.calls[-1]
        self.assertEqual(call[:4], ["gh", "release", "create", "0.7.0.1"])
        for flag in ("--verify-tag", "--notes-file", "--latest=true"):
            self.assertIn(flag, call)
        self.assertNotIn("--generate-notes", call)
        self.assertNotIn("--notes-start-tag", call)
        self.assertEqual(call[call.index("--title") + 1], "0.7.0.1 [mainnet]")
        self.assertNotIn("--prerelease", call)

    def test_release_gh_release_recreate_order_ok(self):
        out = io.StringIO()

        def run(argv):
            if argv[:3] == ["gh", "release", "delete"]:
                out.write("<delete>")
            return runner(argv)

        runner = FakeRunner(
            {("gh", "release", "delete"): "", ("gh", "release", "create"): ""}
        )
        old = rel.ReleaseInfo("0.7.0.1", "0.7.0.1 [decaf]", "old body")
        edit = rel.ReleaseEdit(
            "0.7.0.1", "0.7.0.1 [decaf, mainnet]", "new", "recreate", old
        )
        with contextlib.redirect_stdout(out):
            rel.Gh(run).release_recreate(edit)
        logged = out.getvalue()
        self.assertRegex(
            logged, r"# old release 0\.7\.0\.1: 0\.7\.0\.1 \[decaf\]\nold body\n# end"
        )
        self.assertLess(logged.index("old body"), logged.index("<delete>"))
        self.assertEqual(
            runner.calls[0], ["gh", "release", "delete", "0.7.0.1", "--yes"]
        )
        self.assertEqual(runner.calls[1][:4], ["gh", "release", "create", "0.7.0.1"])
        self.assertIn("--verify-tag", runner.calls[1])

    def test_release_gh_release_recreate_restores_old_on_create_failure_fails(self):
        creates = []

        def run(argv):
            if argv[:3] == ["gh", "release", "create"]:
                creates.append(argv)
                if len(creates) == 1:
                    raise RuntimeError("create failed")
            return runner(argv)

        runner = FakeRunner(
            {("gh", "release", "delete"): "", ("gh", "release", "create"): ""}
        )
        old = rel.ReleaseInfo("0.7.0.1", "0.7.0.1 [decaf]", "old body")
        edit = rel.ReleaseEdit(
            "0.7.0.1", "0.7.0.1 [decaf, mainnet]", "new", "recreate", old
        )
        with (
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(RuntimeError, "create failed"),
        ):
            rel.Gh(run).release_recreate(edit)
        restore = creates[1]
        self.assertEqual(restore[restore.index("--title") + 1], "0.7.0.1 [decaf]")
        self.assertIn("--latest=false", restore)

    def test_release_gh_generate_notes_argv_ok(self):
        prefix = ("gh", "api", "repos/{owner}/{repo}/releases/generate-notes")
        runner = FakeRunner({prefix: json.dumps({"name": "n", "body": "md"})})
        gh = rel.Gh(runner)
        self.assertEqual(gh.generate_notes(t("0.7.0.2"), t("0.7.0.1")), "md")
        self.assertEqual(
            runner.calls[-1],
            [*prefix, "-f", "tag_name=0.7.0.2", "-f", "previous_tag_name=0.7.0.1"],
        )
        gh.generate_notes(t("0.7.0.2"), None)
        self.assertEqual(runner.calls[-1], [*prefix, "-f", "tag_name=0.7.0.2"])


class WhatsChanged(unittest.TestCase):
    SECTION = f"{rel.WHATS_CHANGED_OPEN}\nNEW\n{rel.WHATS_CHANGED_CLOSE}"

    def test_release_whats_changed_replaces_legacy_section_ok(self):
        body = (
            "<!-- publish:decaf prev=none date=d -->\nx\n<!-- /publish:decaf -->\n\n"
            "## What's Changed\n* old in #1\n\n\n**Full Changelog**: https://x/a...b\n"
            "tail\n"
        )
        self.assertEqual(
            rel.with_whats_changed(body, "NEW"),
            "<!-- publish:decaf prev=none date=d -->\nx\n<!-- /publish:decaf -->\n\n"
            f"{self.SECTION}\ntail\n",
        )

    def test_release_whats_changed_replaces_marked_section_ok(self):
        body = f"top\n\n{rel.WHATS_CHANGED_OPEN}\nOLD\n{rel.WHATS_CHANGED_CLOSE}\n"
        self.assertEqual(
            rel.with_whats_changed(body, "NEW"), f"top\n\n{self.SECTION}\n"
        )

    def test_release_whats_changed_marker_in_title_ok(self):
        body = (
            f"top\n\n{rel.WHATS_CHANGED_OPEN}\n* a {rel.WHATS_CHANGED_CLOSE} by @x in #1\n"
            f"* b in #2\n{rel.WHATS_CHANGED_CLOSE}\n"
        )
        self.assertEqual(
            rel.with_whats_changed(body, "NEW"), f"top\n\n{self.SECTION}\n"
        )

    def test_release_whats_changed_legacy_new_contributors_ok(self):
        body = (
            "top\n\n## What's Changed\n* a in #1\n\n## New Contributors\n* @b\n\n"
            "**Full Changelog**: https://x/a...b\n"
        )
        self.assertEqual(
            rel.with_whats_changed(body, "NEW"), f"top\n\n{self.SECTION}\n"
        )

    def test_release_whats_changed_legacy_without_changelog_line_ok(self):
        body = "top\n\n## What's Changed\n* a in #1\n"
        self.assertEqual(rel.with_whats_changed(body, "NEW"), f"top\n\n{self.SECTION}")
        body = "top\n\n## What's Changed\n* a in #1\n\n## Other\nx\n"
        self.assertEqual(
            rel.with_whats_changed(body, "NEW"), f"top\n\n{self.SECTION}\n## Other\nx\n"
        )

    def test_release_whats_changed_appended_when_absent_ok(self):
        self.assertEqual(
            rel.with_whats_changed("top\n", "NEW"), f"top\n\n{self.SECTION}\n"
        )


# EDGE:release-no-tags


class NoTags(unittest.TestCase):
    def test_release_no_tags_ok(self):
        body = rel.render_body(state(), "", REPO)
        self.assertIn("_No tags yet._", body)
        self.assertIn("_None._", body)  # experimental branches


# EDGE:release-anchor-missing


class AnchorMissing(unittest.TestCase):
    def test_release_anchor_missing_ok(self):
        body = rel.render_body(state(anchor=None), "", REPO)
        self.assertIn("No anchor tag", body)


# EDGE:release-anchor-unreachable


class AnchorUnreachable(unittest.TestCase):
    def test_release_anchor_unreachable_ok(self):
        anchor = "c" * 40
        main_commits = [commit(sha="1" * 40, subject="feat: thing (#5)", pr=5)]
        body = rel.render_body(
            state(
                anchor=anchor,
                anchor_reachable=False,
                head_branch="d" * 8 + "e" * 32,
                main_commits=main_commits,
            ),
            "",
            REPO,
        )
        self.assertIn("no longer reachable", body)
        main_section, _, branch_section = body.partition(
            "## Commits on `release-0.6.0`"
        )
        self.assertIn(f"[`{main_commits[0].sha[:8]}`]", main_section)
        self.assertNotIn("no longer reachable", main_section)
        self.assertIn("no longer reachable", branch_section)


# REQ:release-row-status


class RowStatus(unittest.TestCase):
    def test_release_row_status_ok(self):
        open_pr = rel.BackportPr(5, "OPEN", "u")
        closed_pr = rel.BackportPr(5, "CLOSED", "u")
        cases = [
            ((None, True, None), "✅"),
            (("skip", True, None), "✅"),
            (("done", False, None), "☑️"),
            (("skip", False, open_pr), "⏭️"),
            ((None, False, open_pr), "🟨"),
            ((None, False, closed_pr), "⬜"),
            ((None, False, None), "⬜"),
        ]
        for args, expected in cases:
            with self.subTest(args=args):
                self.assertEqual(rel.row_status(*args), expected)

    def test_release_row_has_no_checkbox_ok(self):
        row = rel.render_row(commit(), REPO, mark=None, landed=True, backport=None)
        self.assertTrue(row.startswith("| ✅ | "))


# EDGE:release-html-subject


class HtmlSubject(unittest.TestCase):
    def test_release_html_subject_ok(self):
        row = rel.render_row(
            commit(subject="fix: a<b> & c|d"),
            REPO,
            mark=None,
            landed=False,
            backport=None,
        )
        self.assertIn("a&lt;b&gt; &amp; c\\|d", row)

    def test_release_row_bare_title_ok(self):
        row = rel.render_row(
            commit(subject="[Backport release-0.6.0] fix: thing (#12)", pr=12),
            REPO,
            mark=None,
            landed=False,
            backport=None,
        )
        self.assertTrue(row.endswith("| fix: thing |"))


# EDGE:release-branch-name-escaping


class ExperimentalBranchEscaping(unittest.TestCase):
    def test_release_experimental_branch_escaping_ok(self):
        rendered = rel.render_experimental_branches(
            [("release-0.6.0--<b>evil</b>", "a" * 40)], REPO
        )
        self.assertIn("&lt;b&gt;evil&lt;/b&gt;", rendered)
        self.assertIn("release-0.6.0--%3Cb%3Eevil%3C%2Fb%3E", rendered)
        self.assertNotIn("<b>", rendered)


# EDGE:release-command-extra-args


class CommandExtraArgs(unittest.TestCase):
    def test_release_command_extra_args_fails(self):
        self.assertIsNone(rel.parse_command("/done abc1234 junk"))


# EDGE:release-command-unknown


class CommandUnknown(unittest.TestCase):
    def test_release_command_unknown_ok(self):
        self.assertIsNone(rel.parse_command("/foo bar"))


# EDGE:release-title-malformed


class TitleMalformed(unittest.TestCase):
    def test_release_title_malformed_fails(self):
        trackers = [(1, "Release 0.6"), (2, "Release 0.7.0")]
        gh = rel.Gh(FakeRunner({}))
        args = argparse.Namespace(version=None, ref="refs/heads/main", issue=None)
        plan, had_error = rel.refresh_plan(args, gh, trackers)
        self.assertTrue(had_error)
        self.assertEqual([v for _issue, v in plan], [version(0, 7, 0)])


# EDGE:release-teardown-no-tracker


class TeardownNoTracker(unittest.TestCase):
    def test_release_teardown_no_tracker_ok(self):
        runner = FakeRunner({("gh", "issue", "list"): "[]"})
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(branch="release-0.6.0")
        code = rel.cmd_teardown(args, git, gh)
        self.assertEqual(code, 0)
        self.assertFalse(runner.ran("gh", "issue", "comment"))
        self.assertFalse(runner.ran("gh", "issue", "close"))


# EDGE:release-cut-rerun


class CmdCutRerun(unittest.TestCase):
    def test_release_cmd_cut_rerun_same_sha_ok(self):
        sha = "d" * 40
        runner = FakeRunner(
            {
                ("git", "rev-parse", "main^{commit}"): sha,
                ("git", "ls-remote", "--heads"): f"{sha}\trefs/heads/release-0.6.0\n",
                ("git", "ls-remote", "--tags"): f"{sha}\trefs/tags/0.6.0.0\n",
                ("gh", "label", "create"): "",
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): "[]",
                ("gh", "issue", "create"): "https://example/issues/9",
                ("gh", "workflow", "run"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(version="0.6.0", source_ref="main")
        code = rel.cmd_cut(args, git, gh)
        self.assertEqual(code, 0)
        self.assertFalse(
            runner.ran("git", "push", "origin", f"{sha}:refs/heads/release-0.6.0")
        )

    def test_release_cmd_cut_rerun_diff_sha_fails(self):
        runner = FakeRunner(
            {
                ("git", "rev-parse", "main^{commit}"): "e" * 40,
                ("git", "ls-remote", "--heads"): "deadbeef\trefs/heads/release-0.6.0\n",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(version="0.6.0", source_ref="main")
        code = rel.cmd_cut(args, git, gh)
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("git", "push"))
        self.assertFalse(runner.ran("gh", "issue", "create"))


class CmdCutReusesExistingTracker(unittest.TestCase):
    def test_release_cmd_cut_reuses_existing_tracker_ok(self):
        sha = "d" * 40
        runner = FakeRunner(
            {
                ("git", "rev-parse", "main^{commit}"): sha,
                ("git", "ls-remote", "--heads"): f"{sha}\trefs/heads/release-0.6.0\n",
                ("git", "ls-remote", "--tags"): f"{sha}\trefs/tags/0.6.0.0\n",
                ("gh", "label", "create"): "",
                (
                    "gh",
                    "api",
                    "repos/{owner}/{repo}/releases?per_page=100",
                ): release_row("0.6.0.0") + "\n",
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): json.dumps(
                    [{"number": 7, "title": "Release 0.6.0"}]
                ),
                ("gh", "workflow", "run"): "",
            }
        )
        args = argparse.Namespace(version="0.6.0", source_ref="main")
        self.assertEqual(rel.cmd_cut(args, rel.Git(runner), rel.Gh(runner)), 0)
        self.assertFalse(runner.ran("gh", "issue", "create"))


class CanWrite(unittest.TestCase):
    def _gh(self, error: str) -> "rel.Gh":
        def runner(argv: list[str]) -> str:
            raise RuntimeError(error)

        return rel.Gh(runner)

    def test_release_can_write_not_a_user_fails(self):
        self.assertFalse(self._gh("gh: x is not a user (HTTP 404)").can_write("x"))

    def test_release_can_write_other_error_raises_fails(self):
        with self.assertRaises(RuntimeError):
            self._gh("gh: API rate limit exceeded (HTTP 403)").can_write("x")


class CmdCutFirstRun(unittest.TestCase):
    def test_release_cmd_cut_first_run_ok(self):
        sha = "d" * 40
        runner = FakeRunner(
            {
                ("git", "rev-parse", "main^{commit}"): sha,
                ("git", "ls-remote", "--heads"): "",
                ("git", "ls-remote", "--tags"): "",
                ("git", "push"): "",
                ("gh", "label", "create"): "",
                ("git", "tag", "-a"): "",
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): "[]",
                ("gh", "issue", "create"): "https://example/issues/9",
                ("gh", "workflow", "run"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(version="0.6.0", source_ref="main")
        code = rel.cmd_cut(args, git, gh)
        self.assertEqual(code, 0)
        self.assertIn(
            ["git", "push", "origin", f"{sha}:refs/heads/release-0.6.0"], runner.calls
        )
        self.assertIn(
            ["git", "tag", "-a", "0.6.0.0", sha, "-m", "Release 0.6.0.0"], runner.calls
        )
        create_call = next(
            call for call in runner.calls if call[:3] == ["gh", "issue", "create"]
        )
        self.assertIn("--title", create_call)
        self.assertIn("Release 0.6.0", create_call)
        self.assertFalse(runner.ran("gh", "release"))
        workflow_calls = [
            call for call in runner.calls if call[:3] == ["gh", "workflow", "run"]
        ]
        self.assertEqual(len(workflow_calls), 2)
        self.assertIn(
            ["gh", "workflow", "run", "build.yml", "--ref", "0.6.0.0"], workflow_calls
        )
        self.assertIn(
            ["gh", "workflow", "run", "update-release-tracker.yml", "-f", "issue=9"],
            workflow_calls,
        )


class CmdCutInvalidBump(unittest.TestCase):
    def test_release_cmd_cut_invalid_bump_fails(self):
        runner = FakeRunner(
            {("git", "ls-remote", "--heads"): "a" * 40 + " refs/heads/release-0.6.0\n"}
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(version="0.6.5", source_ref="main")
        code = rel.cmd_cut(args, git, gh)
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("git", "push"))
        self.assertFalse(runner.ran("gh", "issue", "create"))


# REQ:release-teardown-happy-path


class CmdTeardownHappyPath(unittest.TestCase):
    def test_release_cmd_teardown_happy_path_ok(self):
        runner = FakeRunner(
            {
                ("gh", "issue", "list"): json.dumps(
                    [{"number": 9, "title": "Release 0.6.0"}]
                ),
                ("git", "ls-remote", "--tags"): (
                    "a" * 40
                    + " refs/tags/0.6.0.0\n"
                    + "b" * 40
                    + " refs/tags/0.6.0.1\n"
                ),
                ("gh", "issue", "comment"): "",
                ("gh", "issue", "edit"): "",
                ("gh", "issue", "close"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(branch="release-0.6.0")
        code = rel.cmd_teardown(args, git, gh)
        self.assertEqual(code, 0)
        actions = [
            tuple(call[1:3])
            for call in runner.calls
            if call[0] == "gh"
            and call[1] == "issue"
            and call[2] in ("comment", "edit", "close")
        ]
        self.assertEqual(
            actions, [("issue", "comment"), ("issue", "edit"), ("issue", "close")]
        )
        edit_call = next(
            call for call in runner.calls if call[:3] == ["gh", "issue", "edit"]
        )
        self.assertIn("--add-label", edit_call)
        self.assertIn("release-closed", edit_call)
        close_call = next(
            call for call in runner.calls if call[:3] == ["gh", "issue", "close"]
        )
        self.assertIn("--reason", close_call)
        self.assertIn("completed", close_call)


class RenderBodyGolden(unittest.TestCase):
    """One golden-ish assertion: headings, a ticked row, a struck row, a backport link, notes."""

    def test_render_body_golden(self):
        sha_done = "1" * 40
        sha_skip = "2" * 40
        main_commits = [
            commit(sha=sha_done, subject="feat: thing (#10)", pr=10),
            commit(sha=sha_skip, subject="chore: noise (#11)", pr=11),
        ]
        backport = rel.BackportPr(20, "MERGED", "https://example/pr/20")
        tracker = state(
            anchor="a" * 40,
            tags=[("0.6.0.0", "2026-01-01", "a" * 40)],
            main_commits=main_commits,
            marks={sha_skip[:8]: "skip"},
            backport_prs={10: backport},
        )
        body = rel.render_body(tracker, "Kept notes.\n", REPO)

        self.assertIn("## Tag log", body)
        self.assertIn("## Commits on `main` not yet on the branch", body)
        self.assertIn("## Commits on `release-0.6.0`", body)
        self.assertIn("## Experimental branches", body)
        self.assertIn(
            f"| ⬜ | [#10](https://github.com/{REPO}/pull/10) | [`{sha_done[:8]}`]",
            body,
        )  # not landed, not marked done
        self.assertEqual(body.count(rel.STATUS_LEGEND), 1)
        self.assertIn("| 🟣 [#20](https://example/pr/20) | feat: thing |", body)
        self.assertIn(
            f"| ⏭️ | [#11](https://github.com/{REPO}/pull/11) | [`{sha_skip[:8]}`]", body
        )
        self.assertIn("|  | ~~chore: noise~~ |", body)
        self.assertTrue(body.endswith(f"{rel.SENTINEL}\nKept notes.\n"))


if __name__ == "__main__":
    unittest.main()
