"""Tests for `nextest-ci` behaviour that only shows up against the GitHub API.

`gh` is stubbed, so no network. Run with:

    just py::test
"""

import argparse
import importlib.util
import json
import tempfile
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).with_name("nextest-ci")
_spec = importlib.util.spec_from_loader(
    "nextest_ci", SourceFileLoader("nextest_ci", str(SCRIPT))
)
assert _spec is not None
nc = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(nc)

ARTIFACT = "compile-metrics-build"
PAGE_SIZE = 30
# A build run uploads a docker digest artifact per image, so the baseline is not on page one.
EXTRA_ARTIFACTS = [{"name": f"digests-{i}", "expired": False} for i in range(PAGE_SIZE)]


def runs_response(*runs):
    return {"workflow_runs": list(runs)}


def run(run_id, status, created_at):
    return {"id": run_id, "status": status, "created_at": created_at}


class StatsFetch(unittest.TestCase):
    """The baseline artifact is uploaded hours before the run it belongs to finishes."""

    def fetch(self, runs, with_artifact, expired=(), max_runs=5):
        def field(args, name):
            prefix = f"{name}="
            return next(
                (a.split("=", 1)[1] for a in args if a.startswith(prefix)), None
            )

        def gh_json(args):
            path = next(a for a in args if a.startswith("repos/"))
            if path.endswith("/artifacts"):
                run_id = int(path.split("/runs/")[1].split("/")[0])
                listed = list(EXTRA_ARTIFACTS)
                if run_id in with_artifact:
                    listed.append({"name": ARTIFACT, "expired": run_id in expired})
                # The endpoint returns one page of 30 unless `name=` narrows it, and a
                # build run has more artifacts than that.
                wanted = field(args, "name")
                if wanted is not None:
                    listed = [a for a in listed if a["name"] == wanted]
                return {"artifacts": listed[:PAGE_SIZE]}
            # Both filters are applied server-side, so neither is something the test can
            # ignore: `status` decides which runs exist, `per_page` how many come back.
            listed = runs["workflow_runs"]
            status = field(args, "status")
            if status:
                listed = [r for r in listed if r["status"] == status]
            per_page = field(args, "per_page")
            return {"workflow_runs": listed[: int(per_page)] if per_page else listed}

        with tempfile.TemporaryDirectory() as td:
            out = Path(td) / "main.json"
            args = argparse.Namespace(
                workflow="build.yml",
                artifact_name=ARTIFACT,
                out=out,
                max_runs=max_runs,
                concurrency=2,
            )
            with (
                mock.patch.object(nc, "resolve_repo", return_value="owner/repo"),
                mock.patch.object(nc, "gh_json", side_effect=gh_json),
                mock.patch.object(
                    nc, "download_artifact", side_effect=lambda _r, i, _a: {"run": i}
                ),
            ):
                self.assertEqual(nc.cmd_stats_fetch(args), 0)
            return json.loads(out.read_text())["runs"]

    def test_in_progress_run_with_the_artifact_is_used(self):
        found = self.fetch(
            runs_response(run(2, "in_progress", "2026-08-31T10:00:00Z")),
            with_artifact={2},
        )
        self.assertEqual(found, [{"run": 2}])

    def test_newest_first_regardless_of_status(self):
        found = self.fetch(
            runs_response(
                run(1, "completed", "2026-08-28T10:00:00Z"),
                run(2, "in_progress", "2026-08-31T10:00:00Z"),
            ),
            with_artifact={1, 2},
        )
        self.assertEqual(found, [{"run": 2}, {"run": 1}])

    def test_run_without_the_artifact_is_skipped(self):
        found = self.fetch(
            runs_response(run(3, "queued", "2026-08-31T11:00:00Z")),
            with_artifact=set(),
        )
        self.assertEqual(found, [])

    def test_expired_artifact_is_skipped(self):
        """Retention is 90 days and the window reaches past it."""
        found = self.fetch(
            runs_response(run(4, "completed", "2026-05-01T10:00:00Z")),
            with_artifact={4},
            expired={4},
        )
        self.assertEqual(found, [])

    def test_in_progress_runs_do_not_displace_usable_ones(self):
        """test.yml uploads its stats artifact only once every shard is done, so an
        in-progress run of it can occupy a slot but never fill one."""
        listed = [
            run(100 + i, "in_progress", f"2026-08-31T1{i}:00:00Z") for i in range(5)
        ] + [run(200 + i, "completed", f"2026-08-2{i}T10:00:00Z") for i in range(5)]
        found = self.fetch(
            runs_response(*listed),
            with_artifact={200 + i for i in range(5)},
            max_runs=5,
        )
        self.assertEqual(found, [{"run": 204 - i} for i in range(5)])

    def test_window_is_capped_at_max_runs(self):
        listed = [
            run(300 + i, "completed", f"2026-08-2{i}T10:00:00Z") for i in range(6)
        ]
        found = self.fetch(
            runs_response(*listed),
            with_artifact={300 + i for i in range(6)},
            max_runs=3,
        )
        self.assertEqual(len(found), 3)


if __name__ == "__main__":
    unittest.main()
