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
import unittest
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


def version(major=0, minor=6, phase=0) -> "rel.Version":
    return rel.Version(major, minor, phase)


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
        "releases": {},
        "main_commits": [],
        "branch_commits": [],
        "landed_on_branch": set(),
        "landed_on_main": set(),
        "marks": {},
        "backport_prs": {},
        "experimental_branches": [],
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
                ("git", "tag", "-a"): "",
                ("git", "push", "origin", "refs/tags/0.6.0.2"): "",
                ("gh", "release", "create"): "https://example/releases/0.6.0.2",
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
        release_call = next(
            call for call in runner.calls if call[:3] == ["gh", "release", "create"]
        )
        self.assertIn("--prerelease", release_call)
        self.assertNotIn("--latest=true", release_call)
        self.assertTrue(runner.ran("gh", "workflow", "run"))
        self.assertTrue(runner.ran("gh", "issue", "comment"))

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

    def test_release_gh_releases_prefix_ok(self):
        lines = [
            json.dumps({"tag_name": "0.6.1.0", "prerelease": True}),
            json.dumps({"tag_name": "0.6.1.3", "prerelease": False}),
            json.dumps({"tag_name": "0.6.10.0", "prerelease": True}),
            json.dumps({"tag_name": "0.7.0.0", "prerelease": True}),
        ]
        runner = FakeRunner({("gh", "api"): "\n".join(lines) + "\n"})
        self.assertEqual(
            rel.Gh(runner).releases(rel.Version(0, 6, 1)),
            {"0.6.1.0": True, "0.6.1.3": False},
        )
        self.assertIn("--paginate", runner.calls[-1])


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

    def test_release_tag_write_failure_posts_comment_and_reraises_fails(self):
        sha = "c" * 40
        responses = {
            ("gh", "repo", "view"): REPO,
            ("gh", "issue", "view"): json.dumps({"title": "Release 0.6.0", "body": ""}),
            ("git", "fetch"): "",
            ("git", "tag", "--list", "0.6.0.*"): "",
            ("git", "rev-parse", "origin/release-0.6.0"): sha,
            ("git", "tag", "-a"): "",
            ("git", "push", "origin", "refs/tags/0.6.0.0"): "",
            ("gh", "issue", "comment"): "",
        }
        calls: list[list[str]] = []

        def runner(argv: list[str]) -> str:
            calls.append(argv)
            if argv[:3] == ["gh", "release", "create"]:
                raise RuntimeError("gh release create boom")
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
                ("git", "tag", "--list", "0.6.0.*"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        state, _body_before = rel.build_tracker_state(git, gh, 7, version())
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
        body = rel.render_tag_log(tags, {"0.6.3.1": True}, REPO)
        self.assertIn("| GitHub release | Build |", body)
        self.assertIn(
            f"[pre-release](https://github.com/{REPO}/releases/tag/0.6.3.1)", body
        )
        self.assertIn("build.yml?query=branch%3A0.6.3.0", body)
        self.assertIn("| - |", body)
        self.assertIn("[release](", rel.render_tag_log(tags, {"0.6.3.1": False}, REPO))


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
        self.assertTrue(row.startswith("- ✅ [`aaaaaaaa`]"))


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
        self.assertIn("a&lt;b&gt; &amp; c|d", row)


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
                ("gh", "api", "repos/{owner}/{repo}/releases?per_page=100"): "",
                ("gh", "release", "create"): "https://example/releases/0.6.0.0",
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): "[]",
                ("gh", "issue", "create"): "https://example/issues/9",
                ("gh", "issue", "lock"): "",
                ("gh", "api", "repos/{owner}/{repo}/issues/9"): "false",
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


class CmdCutLocksExistingTracker(unittest.TestCase):
    def _cut(self, locked: str) -> FakeRunner:
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
                ): '{"tag_name": "0.6.0.0", "prerelease": true}\n',
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): json.dumps(
                    [{"number": 7, "title": "Release 0.6.0"}]
                ),
                ("gh", "api", "repos/{owner}/{repo}/issues/7"): locked,
                ("gh", "issue", "lock"): "",
                ("gh", "workflow", "run"): "",
            }
        )
        args = argparse.Namespace(version="0.6.0", source_ref="main")
        self.assertEqual(rel.cmd_cut(args, rel.Git(runner), rel.Gh(runner)), 0)
        self.assertFalse(runner.ran("gh", "issue", "create"))
        return runner

    def test_release_cmd_cut_locks_unlocked_tracker_ok(self):
        self.assertIn(["gh", "issue", "lock", "7"], self._cut("false").calls)

    def test_release_cmd_cut_skips_locked_tracker_ok(self):
        self.assertFalse(self._cut("true").ran("gh", "issue", "lock"))


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
                ("gh", "api", "repos/{owner}/{repo}/releases?per_page=100"): "",
                ("gh", "release", "create"): "https://example/releases/0.6.0.0",
                ("git", "tag", "-a"): "",
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): "[]",
                ("gh", "issue", "create"): "https://example/issues/9",
                ("gh", "issue", "lock"): "",
                ("gh", "api", "repos/{owner}/{repo}/issues/9"): "false",
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
        self.assertIn(["gh", "issue", "lock", "9"], runner.calls)
        release_call = next(
            call for call in runner.calls if call[:3] == ["gh", "release", "create"]
        )
        self.assertEqual(release_call[3:5], ["0.6.0.0", "--target"])
        self.assertIn("--prerelease", release_call)
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


class CmdCutSkipsExistingRelease(unittest.TestCase):
    def test_release_cmd_cut_skips_existing_release_ok(self):
        sha = "d" * 40
        runner = FakeRunner(
            {
                ("git", "rev-parse", "main^{commit}"): sha,
                ("git", "ls-remote", "--heads"): f"{sha}\trefs/heads/release-0.6.0\n",
                ("git", "ls-remote", "--tags"): f"{sha}\trefs/tags/0.6.0.0\n",
                ("gh", "label", "create"): "",
                ("gh", "api", "repos/{owner}/{repo}/releases?per_page=100"): (
                    json.dumps({"tag_name": "0.6.0.0", "prerelease": True}) + "\n"
                ),
                ("git", "tag", "--list", "0.6.0.*"): "",
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "list"): "[]",
                ("gh", "issue", "create"): "https://example/issues/9",
                ("gh", "issue", "lock"): "",
                ("gh", "api", "repos/{owner}/{repo}/issues/9"): "false",
                ("gh", "workflow", "run"): "",
            }
        )
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(version="0.6.0", source_ref="main")
        code = rel.cmd_cut(args, git, gh)
        self.assertEqual(code, 0)
        self.assertFalse(runner.ran("gh", "release", "create"))


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
        self.assertIn(f"- ⬜ [`{sha_done[:8]}`]", body)  # not landed, not marked done
        self.assertIn(rel.STATUS_LEGEND, body)
        self.assertIn("[#20](https://example/pr/20) merged", body)
        self.assertIn(f"- ⏭️ [`{sha_skip[:8]}`]", body)
        self.assertIn("~~chore: noise", body)
        self.assertTrue(body.endswith(f"{rel.SENTINEL}\nKept notes.\n"))


if __name__ == "__main__":
    unittest.main()
