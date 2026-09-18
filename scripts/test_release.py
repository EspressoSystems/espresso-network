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


# REQ:release-latest-flag


class LatestFlag(unittest.TestCase):
    def test_release_latest_flag_ok(self):
        self.assertTrue(rel.is_latest(rel.Tag(version(), 1), ["0.5.0.9"]))
        self.assertFalse(rel.is_latest(rel.Tag(version(), 1), ["0.7.0.0"]))
        self.assertTrue(rel.is_latest(rel.Tag(version(), 0), []))


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


# REQ:release-marks-replay


class MarksReplay(unittest.TestCase):
    def test_release_marks_replay_ok(self):
        comments = [
            rel.Comment("/done abc1234", "MEMBER"),
            rel.Comment("/skip def4567", "OWNER"),
            rel.Comment("/unmark abc1234", "COLLABORATOR"),
        ]
        self.assertEqual(rel.marks_from_comments(comments), {"def4567": "skip"})

    def test_release_marks_replay_nonmember_fails(self):
        comments = [rel.Comment("/done abc1234", "NONE")]
        self.assertEqual(rel.marks_from_comments(comments), {})


# REQ:release-log-parse


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


# REQ:release-comments-paginate


class CommentsPaginate(unittest.TestCase):
    def test_release_comments_paginate_ok(self):
        runner = FakeRunner(
            {
                ("gh", "api"): (
                    '{"body": "second", "author_association": "OWNER", '
                    '"created_at": "2026-01-02T00:00:00Z"}\n'
                    '{"body": "first", "author_association": "MEMBER", '
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


# REQ:release-tag-bad-comment


class CmdTagBadComment(unittest.TestCase):
    def test_release_cmd_tag_bad_comment_fails(self):
        runner = FakeRunner({("gh", "repo", "view"): REPO})
        git, gh = rel.Git(runner), rel.Gh(runner)
        args = argparse.Namespace(
            issue=None,
            branch="release-0.6.0",
            comment="/tag please 0.5.0.9",
            explicit=None,
            actor="someone",
            run_url=None,
            dry_run=False,
        )
        code = rel.cmd_tag(args, git, gh)
        self.assertEqual(code, 1)
        self.assertFalse(runner.ran("git", "tag"))
        self.assertFalse(runner.ran("git", "push"))
        self.assertFalse(runner.ran("gh", "release", "create"))


# REQ:release-tag-happy-path


class CmdTagHappyPath(unittest.TestCase):
    def _run_tag(self, universe_tags: str, expected_latest: str) -> FakeRunner:
        sha = "c" * 40
        runner = FakeRunner(
            {
                ("gh", "repo", "view"): REPO,
                ("gh", "issue", "view"): json.dumps(
                    {"title": "Release 0.6.0", "body": ""}
                ),
                ("git", "fetch"): "",
                ("git", "tag", "--list", "0.6.0.*"): "0.6.0.0\n0.6.0.1\n",
                ("git", "tag", "--list", "*"): universe_tags,
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
        self.assertIn(f"--latest={expected_latest}", release_call)
        self.assertTrue(runner.ran("gh", "workflow", "run"))
        self.assertTrue(runner.ran("gh", "issue", "comment"))
        return runner

    def test_release_cmd_tag_happy_path_latest_ok(self):
        self._run_tag("0.6.0.0\n0.6.0.1\n", "true")

    def test_release_cmd_tag_happy_path_not_latest_ok(self):
        self._run_tag("0.6.0.0\n0.6.0.1\n0.7.0.0\n", "false")

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
        git.fetch(["main"])
        self.assertEqual(git.ref("main"), "main")
        self.assertEqual(git.resolve("release-0.6.0"), "b" * 40)
        self.assertEqual(
            git.ls_remote_heads("release-0.6.0--*"), [("release-0.6.0--x", "a" * 40)]
        )
        self.assertFalse(runner.ran("git", "fetch"))


# REQ:release-refresh-write-no-write


class CmdRefreshWriteNoWrite(unittest.TestCase):
    def _responses(self, body_before: str) -> dict[tuple[str, ...], str]:
        return {
            ("gh", "repo", "view"): REPO,
            ("gh", "issue", "list"): "[]",
            ("gh", "issue", "view"): json.dumps(
                {"title": "Release 0.6.0", "body": body_before}
            ),
            ("gh", "api"): "",
            ("gh", "pr", "list"): "[]",
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
                ("gh", "label", "create"): "",
                ("git", "tag", "--list", "0.6.0.0"): "0.6.0.0\n",
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
        self.assertIn(f"- [ ] [`{sha_done[:8]}`]", body)  # not landed, not marked done
        self.assertIn("[#20](https://example/pr/20) merged", body)
        self.assertIn("~~chore: noise", body)
        self.assertTrue(body.endswith(f"{rel.SENTINEL}\nKept notes.\n"))


if __name__ == "__main__":
    unittest.main()
