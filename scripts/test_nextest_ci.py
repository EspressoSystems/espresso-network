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

    def fetch(self, runs, with_artifact):
        def gh_json(args):
            path = next(a for a in args if a.startswith("repos/"))
            if path.endswith("/artifacts"):
                run_id = int(path.split("/runs/")[1].split("/")[0])
                listed = (
                    [*EXTRA_ARTIFACTS, {"name": ARTIFACT, "expired": False}]
                    if run_id in with_artifact
                    else list(EXTRA_ARTIFACTS)
                )
                # The endpoint returns one page of 30 unless `name=` narrows it, and a
                # build run has more artifacts than that.
                wanted = next(
                    (a.split("=", 1)[1] for a in args if a.startswith("name=")), None
                )
                if wanted is not None:
                    listed = [a for a in listed if a["name"] == wanted]
                return {"artifacts": listed[:PAGE_SIZE]}
            # The endpoint filters server-side, so a `status=` the caller sends is not
            # something the test can ignore.
            wanted = next(
                (a.split("=", 1)[1] for a in args if a.startswith("status=")), None
            )
            listed = runs["workflow_runs"]
            if wanted:
                listed = [r for r in listed if r["status"] == wanted]
            return {"workflow_runs": listed}

        with tempfile.TemporaryDirectory() as td:
            out = Path(td) / "main.json"
            args = argparse.Namespace(
                workflow="build.yml",
                artifact_name=ARTIFACT,
                out=out,
                max_runs=5,
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


if __name__ == "__main__":
    unittest.main()
