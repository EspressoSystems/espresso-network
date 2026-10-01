"""Tests for `bench`: rendering, comparison, validity, and the load generator against a fake node.

No network is started.

    just py::test
"""

import argparse
import asyncio
import base64
import contextlib
import dataclasses
import importlib.util
import io
import json
import math
import socket
import statistics
import subprocess
import sys
import tempfile
import threading
import time
import types
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib.machinery import SourceFileLoader
from pathlib import Path
from unittest import mock

SCRIPT = Path(__file__).with_name("bench")
_spec = importlib.util.spec_from_loader("bench", SourceFileLoader("bench", str(SCRIPT)))
assert _spec is not None
bench = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(bench)


def quantiles(p50, p99):
    return {"n": 100, "mean": p50, "p50": p50, "p95": p99, "p99": p99, "max": p99}


def node(cpu):
    return {
        "role": "validator, sqlite",
        "decided_height_end": 500,
        "decided_blocks": 400,
        "view_lag_end": 0,
        "cpu_cores": cpu,
        "rss_peak_bytes": 500_000_000,
        "tokio_busy_frac": 0.2,
        "ops": {
            "consensus_storage_append_da": {
                "per_s": 2.0,
                "mean_ms": 5.0,
                "p50_ms": 4.0,
                "p99_ms": 20.0,
                "busy_frac": 0.01,
            }
        },
    }


def step(rate, decided=None, consensus=(), query=(), consensus_p50=900.0):
    """A step at `rate` MB/s that decides `decided` (default: all of it)."""
    return {
        "rate_mb_s": rate,
        "refine": False,
        "t_start": 0.0,
        "t_mid": 15.0,
        "t_end": 30.0,
        "decided_mb_s": rate if decided is None else decided,
        "timeouts": 0,
        "consensus_latency_ms": quantiles(consensus_p50, 2000.0),
        "query_lag_ms": quantiles(200.0, 400.0),
        "query_lag_slope_ms_s": 0.0,
        "latency_ms": quantiles(1200.0, 2500.0),
        "mean_view_ms": 500.0,
        "cpu_s_per_mb": 0.5,
        "node_cpu": {"node0": 1.0, "node1": 0.5, "node2": 0.5},
        "postgres_cpu": 0.3,
        "consensus_fails": list(consensus),
        "query_fails": list(query),
        "passed": not consensus and not query,
    }


def make_result(steps=None, steal=0.0, config_hash="abc123"):
    """Steps at 4, 6 and 8 MB/s, the last failing on decided: capacity 6 MB/s."""
    if steps is None:
        steps = [
            step(4.0),
            step(6.0),
            step(8.0, 7.0, consensus=["decided 88% of offered"]),
        ]
    result = {
        "schema_version": bench.SCHEMA_VERSION,
        "run": {
            "sha": "0123456789abcdef",
            "ref": "refs/heads/main",
            "event": "push",
            "run_id": "1",
            "run_url": "https://github.com/o/r/actions/runs/1",
            "pr": None,
            "started_at": "2026-01-01T00:00:00+00:00",
            "wall_s": 400.0,
            "ready_s": 40.0,
            "local": False,
            "teardown": [],
        },
        "runner": {
            "cpu_model": "Test CPU",
            "nproc": 4,
            "affinity": 4,
            "cgroup_cpu_max": None,
            "mhz": [3000.0],
            "flags": [],
            "mem_total_bytes": 16_000_000_000,
            "kernel": "6",
            "image_os": "ubuntu24",
            "image_version": "1",
            "runner_name": "r",
        },
        "calibration": {
            "before": {
                "sha256_1t_mb_s": 2000.0,
                "sha256_mt_mb_s": 8000.0,
                "fsync_per_s": 300.0,
            },
            "after": {
                "sha256_1t_mb_s": 2000.0,
                "sha256_mt_mb_s": 8000.0,
                "fsync_per_s": 300.0,
            },
            "drift_pct": 0.0,
        },
        "config": {
            "workers": 6,
            "tx_size": 100_000,
            "submit_nodes": 3,
            "steps": [4.0, 6.0, 8.0],
            "step_s": 30,
            "warmup_s": 60,
            "cap_s": 5.0,
            "latency_target_ms": 1000,
            "query_lag_target_ms": 1000,
        },
        "config_hash": config_hash,
        "window": {"t0": 0.0, "t1": 180.0, "height_start": 100, "height_end": 500},
        "steps": steps,
        "capacity": bench.capacity(steps),
        "nodes": {"node0": node(1.0), "node1": node(0.5), "node2": node(0.5)},
        "processes": {
            "postgres": {"cpu_cores_mean": 0.3, "cpu_s": 54.0, "rss_peak_bytes": 10}
        },
        "host": {
            "util_mean": 0.5,
            "util_max": 0.7,
            "steal_pct": steal,
            "iowait_pct": 1.0,
            "mem_avail_min_bytes": 8_000_000_000,
        },
        "load": {
            "submitted": 1000,
            "included": 990,
            "timeouts": 10,
            "submit_errors": 0,
            "max_in_flight": 48,
            "cap_waits": 0,
            "missing_payloads": [],
            "tracker_lag_ms": quantiles(50.0, 100.0),
            "drain_s": 3.0,
            "refine_skipped": False,
        },
        "stake_table": ["0x1", "0x1", "0x1"],
        "validity": {"valid": True, "noisy": False, "reasons": []},
    }
    result["validity"] = bench.check_validity(result, {n: 1.0 for n in bench.NODES})
    return result


def compare(current, runs, error=None, source="main"):
    return bench.compare(current, {"runs": runs, "error": error, "source": source})


def row(comparison, label):
    return next(r for r in comparison["capacity"] if r["label"] == label)


def step_row(comparison, rate, label):
    by_rate = next(c for c in comparison["steps"] if c["rate_mb_s"] == rate)
    return next(r for r in by_rate["rows"] if r["label"] == label)


class RenderTest(unittest.TestCase):
    def test_step_table_with_deltas(self):
        baseline = [make_result() for _ in range(5)]
        current = make_result(
            [
                step(4.0),
                step(6.0, consensus_p50=1200.0),
                step(8.0, 7.0, consensus=["x"]),
            ]
        )
        summary = bench.render(current, compare(current, baseline))
        self.assertIn("| main median (n=5) |", summary)
        self.assertIn(
            "| 6 | 6 MB/s | **1200 ms (+33%)** | 2000 ms | 200 ms | 1200 ms | 0.5 s/MB "
            "| pass | 5 |",
            summary,
        )
        self.assertIn("Test CPU", summary)

    def test_capacity_first_detail_last(self):
        summary = bench.render(make_result(), None)
        order = [
            "Capacity **6 MB/s**: consensus limits (decided 88% of offered at 8 MB/s).",
            "Baseline: none given",
            "### Load steps",
            "| MB/s | decided |",
            "<details><summary>Step details</summary>",
            "\n### Load\n",
            "- benchmark tracker lag: p50 50, p95 100, p99 100",
        ]
        self.assertEqual(sorted(order, key=summary.index), order)
        self.assertNotIn("| delta |", summary)

    def test_rates_not_reached(self):
        current = make_result([step(4.0), step(6.0, 5.0, consensus=["x"]), step(5.0)])
        comparison = compare(current, [make_result()] * 3)
        eight = next(c for c in comparison["steps"] if c["rate_mb_s"] == 8.0)
        self.assertEqual(eight["n"], 3)
        summary = bench.render(current, comparison)
        self.assertIn("| 8 | | | | | | | **not reached** | 3 |", summary)
        self.assertIn("| 5 | 5 MB/s | 900 ms |", summary)

    def test_baseline_only_refine_rates_are_left_out(self):
        refine = step(7.0)
        refine["refine"] = True
        baseline = make_result([step(4.0), step(6.0), step(8.0, 6.0, ["x"]), refine])
        summary = bench.render(make_result(), compare(make_result(), [baseline] * 3))
        self.assertNotRegex(summary, r"(?m)^\| 7 ")

    def test_reference_baseline_header(self):
        comparison = compare(make_result(), [make_result()], source="reference")
        self.assertIn(
            "| verdict | reference runs |", bench.render(make_result(), comparison)
        )

    def test_fail_rules_short_in_the_table_full_in_details(self):
        summary = bench.render(make_result(), None)
        self.assertIn("| fail: decided |", summary)
        self.assertIn("| decided 88% of offered |", summary)

    def test_ops_table_drops_duplicates(self):
        current = make_result()
        op = current["nodes"]["node0"]["ops"]["consensus_storage_append_da"]
        current["nodes"]["node0"]["ops"] |= {
            "consensus_internal_append_da2_duration": op,
            "consensus_decide_processor_process_duration": op,
        }
        table = "\n".join(bench.ops_table(current))
        self.assertIn("storage_append_da", table)
        self.assertNotIn("append_da2", table)
        self.assertNotIn("decide_processor", table)

    def test_status_names_the_pr(self):
        current = make_result()
        current["run"] |= {"pr": 42, "event": "workflow_dispatch"}
        self.assertIn("`0123456789` PR #42, config", bench.render(current, None))

    def test_no_baseline(self):
        current = make_result()
        comparison = compare(current, [])
        self.assertEqual(comparison["n"], 0)
        self.assertIn(
            "Baseline: no main runs to compare against.",
            bench.render(current, comparison),
        )
        self.assertIn(
            "Baseline: none given, no main runs to compare against.",
            bench.render(current, None),
        )

    def test_failed_fetch_is_not_no_baseline(self):
        current = make_result()
        summary = bench.render(current, compare(current, [], "OSError: timed out"))
        self.assertIn(
            "Baseline: fetching main runs failed: OSError: timed out", summary
        )
        self.assertNotIn("no main runs", summary)

    def test_load_baseline_single_result(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "result.json"
            path.write_text(json.dumps(make_result()))
            self.assertEqual(len(bench.load_baseline(path)["runs"]), 1)
            self.assertEqual(bench.load_baseline(path)["source"], "reference")
            path.write_text(json.dumps({"runs": []}))
            self.assertEqual(bench.load_baseline(path)["source"], "main")
            path.write_text("[]")
            with self.assertRaises(ValueError):
                bench.load_baseline(path)

    def test_compare_cli_with_empty_runs_exits_zero(self):
        with tempfile.TemporaryDirectory() as tmp:
            result, baseline = Path(tmp) / "result.json", Path(tmp) / "baseline.json"
            result.write_text(json.dumps(make_result()))
            baseline.write_text(json.dumps({"runs": [], "error": "none found"}))
            args = argparse.Namespace(result=result, baseline=baseline)
            with mock.patch("builtins.print"):
                self.assertEqual(bench.cmd_compare(args), 0)


class BaselineLabelTest(unittest.TestCase):
    def test_main_runs(self):
        noisy = make_result(steal=8.0)
        comparison = compare(make_result(), [make_result()] * 3 + [noisy])
        self.assertEqual(bench.baseline_label(comparison), "main median (n=3)")
        self.assertEqual(
            bench.baseline_line(comparison),
            "Baseline: median of 3 main runs: "
            + ", ".join(
                [
                    "[`0123456789`](https://github.com/o/r/actions/runs/1) 2026-01-01 00:00 UTC"
                ]
                * 3
            )
            + " (excluded: 1 noisy).",
        )

    def test_reference_run(self):
        reference = make_result()
        reference["run"] |= {"run_url": None, "local": True, "event": "local"}
        reference["run"]["ref"] = "my-branch"
        comparison = compare(make_result(), [reference], source="reference")
        self.assertEqual(bench.baseline_label(comparison), "reference `0123456789`")
        self.assertEqual(
            bench.baseline_line(comparison),
            "Baseline: reference run `0123456789` 2026-01-01 00:00 UTC, local my-branch.",
        )

    def test_reference_not_comparable(self):
        other = make_result(config_hash="other")
        comparison = compare(make_result(), [other], source="reference")
        self.assertEqual(
            bench.baseline_line(comparison),
            "Baseline: reference run not comparable (excluded: 1 other config).",
        )

    def test_none(self):
        self.assertEqual(
            bench.baseline_line(None),
            "Baseline: none given, no main runs to compare against.",
        )


class CompareTest(unittest.TestCase):
    def test_within_threshold_is_same(self):
        current = make_result([step(4.0), step(6.0, 5.8), step(8.0, 7.0, ["x"])])
        comparison = compare(current, [make_result()])
        self.assertEqual(step_row(comparison, 6.0, "decided")["verdict"], "same")
        self.assertEqual(row(comparison, "capacity")["verdict"], "same")

    def test_lower_capacity_is_worse(self):
        current = make_result([step(4.0), step(6.0, 5.0, ["x"]), step(5.0)])
        comparison = compare(current, [make_result()] * 3)
        capacity = row(comparison, "capacity")
        self.assertEqual(
            (capacity["current"]["mb_s"], capacity["baseline"]["mb_s"]), (5.0, 6.0)
        )
        self.assertEqual(capacity["verdict"], "worse")

    def test_capacity_below_the_first_step_is_worse(self):
        current = make_result([step(4.0, 1.0, ["x"])])
        comparison = compare(current, [make_result()] * 3)
        capacity = row(comparison, "capacity")
        self.assertEqual(capacity["verdict"], "worse")
        self.assertIn(
            "| capacity | < 4 MB/s | 6 MB/s | 3 |  | **worse** (±1 MB/s) |",
            bench.render(current, comparison),
        )

    def test_lower_bound_below_the_baseline_is_inconclusive(self):
        higher = make_result([step(4.0), step(6.0), step(8.0), step(10.0, 5.0, ["x"])])
        current = make_result([step(4.0), step(6.0)])
        comparison = compare(current, [higher] * 3)
        self.assertEqual(row(comparison, "capacity")["verdict"], "inconclusive")
        self.assertIn(
            "| capacity | >= 6 MB/s | 8 MB/s |", bench.render(current, comparison)
        )

    def test_baseline_runs_below_the_first_step_count(self):
        below = make_result([step(4.0, 1.0, ["x"])])
        comparison = compare(make_result(), [below, below, make_result()])
        capacity = row(comparison, "capacity")
        self.assertEqual(capacity["n"], 3)
        self.assertEqual(capacity["baseline"], {"mb_s": None, "bounded": True})
        self.assertEqual(capacity["verdict"], "better")

    def test_capacity_changes_by_ramp_resolution(self):
        # Steps 2 MB/s apart: a refine step is 1 MB/s away.
        refined = make_result([step(4.0), step(6.0), step(8.0, 6.0, ["x"]), step(7.0)])
        self.assertEqual(
            row(compare(refined, [make_result()] * 3), "capacity")["verdict"], "better"
        )
        self.assertEqual(
            row(compare(make_result(), [make_result()] * 3), "capacity")["verdict"],
            "same",
        )

    def test_one_refine_step_shift_is_not_lost_to_float_error(self):
        resolution = bench.ramp_resolution((0.1, 0.3))
        current, baseline = (
            {"mb_s": 0.2, "bounded": True},
            {"mb_s": 0.3, "bounded": True},
        )
        verdict = bench.compare_capacity("x", current, [baseline], resolution, False)
        self.assertEqual(verdict["verdict"], "worse")

    def test_steps_compare_only_against_runs_at_that_rate(self):
        short = make_result([step(4.0, consensus_p50=500.0), step(6.0, 3.0, ["x"])])
        comparison = compare(make_result(), [short, make_result()])
        six = step_row(comparison, 4.0, "consensus p50")
        self.assertEqual(six["baseline"], 700.0)
        eight = next(c for c in comparison["steps"] if c["rate_mb_s"] == 8.0)
        self.assertEqual(eight["n"], 1)

    def test_spread_widens_threshold(self):
        history = [
            make_result([step(4.0, consensus_p50=v)])
            for v in (600, 900, 1200, 750, 1050)
        ]
        verdict = step_row(
            compare(make_result([step(4.0, consensus_p50=1200.0)]), history),
            4.0,
            "consensus p50",
        )
        self.assertGreater(verdict["threshold_pct"], 15)
        self.assertEqual(verdict["verdict"], "same")

    def test_excludes_noisy_invalid_and_other_config(self):
        noisy = make_result(steal=8.0)
        invalid = make_result()
        invalid["validity"] = {"valid": False, "noisy": False, "reasons": ["x"]}
        other = make_result(config_hash="other")
        comparison = compare(make_result(), [noisy, invalid, other, make_result()])
        self.assertEqual(comparison["n"], 1)
        self.assertEqual(
            comparison["excluded"], {"noisy": 1, "invalid": 1, "other config": 1}
        )

    def test_excludes_other_runner(self):
        other_cpu = make_result()
        other_cpu["runner"]["cpu_model"] = "Other CPU"
        slower = make_result()
        slower["calibration"]["before"]["sha256_1t_mb_s"] = 1700.0
        close = make_result()
        close["calibration"]["before"]["sha256_1t_mb_s"] = 1900.0
        comparison = compare(make_result(), [other_cpu, slower, close])
        self.assertEqual(comparison["n"], 1)
        self.assertEqual(
            comparison["excluded"], {"other runner": 1, "other calibration": 1}
        )
        self.assertIn(
            "Baseline: no main runs to compare against (excluded: 1 other runner).",
            bench.render(make_result(), compare(make_result(), [other_cpu])),
        )

    def test_noisy_current_is_inconclusive(self):
        current = make_result([step(4.0), step(6.0, 1.0, ["x"])], steal=8.0)
        comparison = compare(current, [make_result()])
        self.assertEqual(row(comparison, "capacity")["verdict"], "inconclusive")

    def test_other_schema_is_excluded(self):
        old = make_result()
        old["schema_version"] = 1
        self.assertEqual(compare(make_result(), [old])["excluded"], {"other schema": 1})


class ValidityTest(unittest.TestCase):
    def test_steal_marks_noisy(self):
        validity = make_result(steal=8.0)["validity"]
        self.assertTrue(validity["valid"])
        self.assertTrue(validity["noisy"])
        self.assertIn("steal 8.0%", validity["reasons"][0])

    def test_drift_marks_noisy(self):
        result = make_result()
        result["calibration"]["drift_pct"] = -15.0
        self.assertTrue(
            bench.check_validity(result, {n: 1.0 for n in bench.NODES})["noisy"]
        )

    def test_unequal_stake_and_stalled_node_are_invalid(self):
        result = make_result()
        result["stake_table"] = ["0x1", "0x2", "0x1"]
        result["nodes"]["node2"]["decided_blocks"] = 0
        validity = bench.check_validity(
            result, {"node0": 1.0, "node1": 0.5, "node2": 1.0}
        )
        self.assertFalse(validity["valid"])
        self.assertEqual(len(validity["reasons"]), 3)

    def test_lagging_tracker_marks_noisy(self):
        result = make_result()
        result["load"]["tracker_lag_ms"] = quantiles(300.0, 1500.0)
        validity = bench.check_validity(result, {n: 1.0 for n in bench.NODES})
        self.assertTrue(validity["noisy"])
        self.assertIn("benchmark tracker behind", validity["reasons"][0])

    def test_stalled_step_is_invalid(self):
        result = make_result([step(4.0), step(6.0, 0.0, ["decided 0% of offered"])])
        validity = bench.check_validity(result, {n: 1.0 for n in bench.NODES})
        self.assertFalse(validity["valid"])
        self.assertEqual(validity["reasons"], ["the 6 MB/s step decided nothing"])


class ReportOnlyTest(unittest.TestCase):
    def report(self, current):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "config.json").write_text(
                json.dumps(dataclasses.asdict(bench.BenchConfig()))
            )
            baseline = out / "baseline.json"
            baseline.write_text(json.dumps({"runs": [make_result()] * 3}))
            with (
                mock.patch.object(bench, "analyze", return_value=current),
                mock.patch("builtins.print"),
                self.assertLogs(bench.log),
            ):
                code = bench.write_report(out, bench.load_baseline(baseline))
            return code, (out / "summary.md").read_text()

    def test_regression_still_exits_zero(self):
        code, summary = self.report(make_result([step(4.0, 3.0, ["x"])]))
        self.assertEqual(code, 0)
        self.assertIn("**worse**", summary)

    def test_invalid_run_exits_one(self):
        current = make_result()
        current["validity"] = {"valid": False, "noisy": False, "reasons": ["no blocks"]}
        self.assertEqual(self.report(current)[0], 1)


class PreflightTest(unittest.TestCase):
    def test_busy_port_refuses(self):
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            sock.listen()
            port = sock.getsockname()[1]
            self.assertEqual(
                bench.preflight_problems((port,), ()), [f"port {port} is in use"]
            )

    def test_bad_baseline_fails_before_starting_network(self):
        with tempfile.TemporaryDirectory() as tmp:
            baseline = Path(tmp) / "baseline.json"
            baseline.write_text('{"runs": [')
            args = bench.parse_args(
                ["run", "--bin-dir", "/nonexistent", "--baseline", str(baseline)]
            )
            with (
                mock.patch.object(
                    bench, "start_network", side_effect=AssertionError("started")
                ),
                self.assertRaises(json.JSONDecodeError),
            ):
                bench.cmd_run(args)

    def test_run_refuses_without_starting_network(self):
        args = bench.parse_args(["run", "--bin-dir", "/nonexistent"])
        with (
            mock.patch.object(
                bench, "preflight_problems", return_value=["port 1 is in use"]
            ),
            mock.patch.object(
                bench, "start_network", side_effect=AssertionError("started")
            ),
            self.assertLogs(bench.log, "ERROR") as logs,
        ):
            self.assertEqual(bench.cmd_run(args), 2)
        self.assertIn("scripts/cleanup-process-compose", "\n".join(logs.output))


class ReadRetryTest(unittest.TestCase):
    def test_retries_failed_reads(self):
        pool = mock.Mock(closed=threading.Event())
        pool.request.side_effect = [OSError("timed out"), (503, b""), (200, b"7")]
        with (
            mock.patch.object(bench, "READ_RETRY_S", 0),
            self.assertLogs(bench.log, "WARNING"),
        ):
            self.assertEqual(bench.query_height(pool, "http://x"), 7)
        self.assertEqual(pool.request.call_count, 3)

    def test_gives_up_after_the_deadline(self):
        pool = mock.Mock(closed=threading.Event())
        pool.request.side_effect = OSError("timed out")
        with (
            mock.patch.object(bench, "READ_RETRY_S", 0.01),
            mock.patch.object(bench, "READ_DEADLINE_S", 0.05),
            self.assertLogs(bench.log, "WARNING"),
            self.assertRaises(bench.NetworkError),
        ):
            bench.query_height(pool, "http://x")

    def test_closing_the_pool_stops_retries(self):
        pool = bench.HttpPool()
        threading.Timer(0.2, pool.close).start()
        start = time.time()
        with (
            mock.patch.object(pool, "request", side_effect=OSError("timed out")),
            mock.patch.object(bench, "READ_RETRY_S", 0.05),
            self.assertLogs(bench.log, "DEBUG") as logs,
            self.assertRaises(bench.NetworkError),
        ):
            bench.query_height(pool, "http://x")
        self.assertLess(time.time() - start, 1.0)
        levels = [record.levelname for record in logs.records]
        self.assertEqual(levels[0], "WARNING")
        self.assertEqual(set(levels[1:]), {"DEBUG"})

    def test_not_found_is_not_retried(self):
        pool = mock.Mock(closed=threading.Event())
        pool.request.return_value = (404, b"")
        self.assertIsNone(bench.block_payload(pool, "http://x", 3))
        self.assertEqual(pool.request.call_count, 1)


class CmdRunTest(unittest.TestCase):
    """`cmd_run` with the runner, calibration and storage stubbed out."""

    def cmd_run(self, tmp, **patches):
        out, storage = Path(tmp) / "out", Path(tmp) / "storage"
        storage.mkdir()
        args = bench.parse_args(["run", "--bin-dir", "/nonexistent", "--out", str(out)])
        calib = {"sha256_1t_mb_s": 1.0, "sha256_mt_mb_s": 1.0, "fsync_per_s": 1.0}
        stubs = {
            "preflight_problems": mock.Mock(return_value=[]),
            "collect_sysinfo": mock.Mock(return_value=(make_result()["runner"], {})),
            "calibrate": mock.Mock(return_value=calib),
            "make_storage": mock.Mock(return_value=storage),
            "remove_storage": mock.Mock(),
        } | patches
        with contextlib.ExitStack() as stack:
            for name, stub in stubs.items():
                stack.enter_context(mock.patch.object(bench, name, stub))
            stack.enter_context(mock.patch("builtins.print"))
            code = bench.cmd_run(args)
        return code, out

    def test_unexpected_error_tears_down_and_writes_failure_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            net = bench.Network(proc=mock.Mock(), out=Path(tmp), storage=Path(tmp))
            teardown = mock.Mock(return_value=[])
            with self.assertLogs(bench.log, "ERROR"):
                code, out = self.cmd_run(
                    tmp,
                    start_network=mock.Mock(return_value=net),
                    sample_metrics=mock.Mock(),
                    sample_host=mock.Mock(),
                    wait_ready=mock.Mock(side_effect=ValueError("bad height")),
                    teardown=teardown,
                )
            self.assertEqual(code, 1)
            teardown.assert_called_once_with(net)
            run = json.loads((out / "run.json").read_text())
            self.assertEqual(run["error"], "ValueError: bad height")
            self.assertIn(
                "**invalid**: ValueError: bad height", (out / "summary.md").read_text()
            )

    def test_metadata_is_read_before_the_run(self):
        order = []
        meta = make_result()["run"]

        def environment_meta(pr):
            order.append("meta")
            return meta

        def drive_network(*_):
            order.append("run")
            return {"error": "stalled", "ready_s": None, "teardown": []}

        with tempfile.TemporaryDirectory() as tmp:
            code, out = self.cmd_run(
                tmp,
                environment_meta=mock.Mock(side_effect=environment_meta),
                drive_network=mock.Mock(side_effect=drive_network),
            )
            self.assertEqual(code, 1)
            self.assertEqual(order, ["meta", "run"])
            self.assertEqual(json.loads((out / "run.json").read_text())["meta"], meta)


class TeardownTest(unittest.TestCase):
    def test_hung_stop_is_force_killed_and_listed(self):
        """TEST:bench-teardown-hang-ok: every step is bounded and a failing step skips none."""
        hung = subprocess.Popen(
            [
                sys.executable,
                "-c",
                (
                    "import signal, time; signal.signal(signal.SIGTERM, signal.SIG_IGN);"
                    " print(flush=True); time.sleep(60)"
                ),
            ],
            stdout=subprocess.PIPE,
            start_new_session=True,
        )
        assert hung.stdout is not None
        hung.stdout.readline()
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "logs").mkdir()
            net = bench.Network(proc=hung, out=Path(tmp), storage=Path(tmp))
            timeout = subprocess.TimeoutExpired("cleanup", 1)
            with (
                mock.patch.object(bench, "STOP_TIMEOUT_S", 0.5),
                mock.patch.object(bench.subprocess, "run", side_effect=timeout),
                mock.patch.object(
                    bench, "own_processes", side_effect=[[], [(1, "gone")]]
                ),
                mock.patch.object(bench.os, "kill", side_effect=ProcessLookupError),
                self.assertLogs(bench.log, "WARNING"),
            ):
                notes = bench.teardown(net)
        self.assertIsNotNone(hung.poll())
        self.assertEqual(
            notes,
            [
                "process-compose did not stop within 0.5 s",
                "cleanup-process-compose ran over 60 s",
            ],
        )


class LabelTest(unittest.TestCase):
    def test_pid_relabelled_after_exec(self):
        cache = {}
        label = mock.patch.object(
            bench,
            "process_label",
            side_effect=lambda pid, comm, *_: bench.PROCESS_LABELS.get(comm),
        )
        with label:
            for comm, want in (("bash", []), ("anvil", [(7, "anvil")])):
                with mock.patch.object(bench, "proc_comms", return_value=[(7, comm)]):
                    self.assertEqual(
                        list(bench.labelled_processes(cache, Path("/"))), want
                    )


class FakeNode(ThreadingHTTPServer):
    """Submit, block height and payload endpoints; `include` decides whether blocks carry the
    submitted transactions. A submit takes `accept_delay` s before the transaction is taken
    and `reply_delay` s after. Payloads in `lost` are never served, those in `late` only
    `late[height]` s after their block was made; every payload answer takes `payload_delay` s.
    The query API shows a block `query_lag` s after the validator status API. A block takes at
    most `block_txs` transactions.

    A node made with `chain=other` is another node of `other`'s network: it shares the chain
    and the pending transactions but takes its own submits. Each node has a key, the chain
    a view per block, and the node at index `view % nodes` leads the view."""

    def __init__(
        self,
        include,
        lost=frozenset(),
        accept_delay=0.0,
        reply_delay=0.0,
        late=None,
        payload_delay=0.0,
        query_lag=0.0,
        block_txs=None,
        chain=None,
    ):
        super().__init__(("127.0.0.1", 0), FakeHandler)
        self.query_lag = query_lag
        self.block_txs = block_txs
        self.include = include
        self.lost = lost
        self.late = late or {}
        self.accept_delay = accept_delay
        self.reply_delay = reply_delay
        self.payload_delay = payload_delay
        self.submits = []
        self.max_outstanding = 0
        if chain is None:
            self.lock = threading.Lock()
            self.pending = []
            self.blocks = [b""]
            self.made = [time.time()]
            self.keys = []
            self.stop = threading.Event()
        else:
            self.lock = chain.lock
            self.pending = chain.pending
            self.blocks = chain.blocks
            self.made = chain.made
            self.keys = chain.keys
            self.stop = chain.stop
        self.key = f"BLS_VER_KEY~fake{len(self.keys)}"
        self.keys.append(self.key)

    @property
    def url(self):
        return f"http://127.0.0.1:{self.server_address[1]}"

    def view(self):
        return len(self.blocks) - 1

    def leader(self, view):
        return self.keys[view % len(self.keys)]

    def produce(self):
        while not self.stop.wait(0.05):
            with self.lock:
                taken = self.pending[: self.block_txs] if self.include else []
                if self.include:
                    del self.pending[: len(taken)]
                self.blocks.append(b"".join(taken))
                self.made.append(time.time())


class FakeHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    # Headers and body go out as separate writes; with Nagle each response waits for a
    # delayed ACK.
    disable_nagle_algorithm = True
    server: FakeNode

    def log_message(self, format, *args):
        pass

    def reply(self, status, body):
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        node = self.server
        tx = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        time.sleep(node.accept_delay)
        with node.lock:
            node.pending.append(base64.b64decode(tx["payload"]))
            node.submits.append(time.time())
            node.max_outstanding = max(node.max_outstanding, len(node.pending))
        time.sleep(node.reply_delay)
        self.reply(200, "TX~fake")

    def do_GET(self):
        node = self.server
        if self.path.startswith("/v1/availability/"):
            time.sleep(node.payload_delay)
        with node.lock:
            if self.path == "/v1/node/block-height":
                shown = time.time() - node.query_lag
                return self.reply(200, sum(1 for t in node.made if t <= shown))
            if self.path == "/v1/status/block-height":
                return self.reply(200, len(node.blocks) - 1)
            if self.path == "/v1/status/keys":
                return self.reply(200, {"consensus_key": node.key})
            if self.path.startswith("/v1/status/upcoming-leaders/"):
                count = int(self.path.rsplit("/", 1)[1])
                view = node.view()
                return self.reply(
                    200,
                    {
                        "view": view,
                        "leaders": [
                            {"view": v, "key": node.leader(v)}
                            for v in range(view + 1, view + 1 + count)
                        ],
                    },
                )
            if self.path == "/v1/status/metrics":
                decided = sum(len(block) for block in node.blocks)
                return self.reply(
                    200,
                    f"consensus_finalized_bytes_sum {decided}\n"
                    "consensus_number_of_timeouts 0\n".encode(),
                )
            height = int(self.path.removeprefix("/v1/availability/payload/"))
            if height >= len(node.blocks) or height in node.lost:
                return self.reply(404, "not found")
            if time.time() - node.made[height] < node.late.get(height, 0.0):
                return self.reply(404, "not found")
            raw = base64.b64encode(node.blocks[height]).decode()
        self.reply(200, {"data": {"raw_payload": raw, "ns_table": {"bytes": ""}}})


class LoadTest(unittest.TestCase):
    def run_load(
        self,
        include,
        duration,
        lost=frozenset(),
        nodes=1,
        accept_delay=0.0,
        reply_delay=0.0,
        late=None,
        payload_delay=0.0,
        query_lag=0.0,
        block_txs=None,
        rate=0.02,
        cap_txs=1000,
        peers=0,
        **cfg,
    ):
        """One step of `duration` s at `rate` MB/s with at most `cap_txs` in flight, no
        warmup, unless `cfg` sets `steps`. The `nodes` urls all name one node, plus `peers`
        more nodes of its network; the peers end up in `self.peers`."""
        node = FakeNode(
            include,
            lost=lost,
            accept_delay=accept_delay,
            reply_delay=reply_delay,
            late=late,
            payload_delay=payload_delay,
            query_lag=query_lag,
            block_txs=block_txs,
        )
        self.peers = [FakeNode(include, chain=node) for _ in range(peers)]
        threads = [
            threading.Thread(target=node.serve_forever),
            threading.Thread(target=node.produce),
            *(threading.Thread(target=peer.serve_forever) for peer in self.peers),
        ]
        for thread in threads:
            thread.start()
        config = bench.BenchConfig(
            **{
                "tx_size": 1000,
                "workers": 3,
                "steps": (rate,),
                "step_s": duration,
                "warmup_s": 0,
                "cap_s": cap_txs * 1000 / (rate * 1e6),
            }
            | cfg
        )
        try:
            with tempfile.TemporaryDirectory() as tmp:
                start = time.time()
                asyncio.run(
                    bench.generate_load(
                        config,
                        [node.url] * nodes + [peer.url for peer in self.peers],
                        node.url,
                        [node.url, node.url],
                        Path(tmp),
                    )
                )
                lines = (Path(tmp) / "load.jsonl").read_text().splitlines()
                txs = [json.loads(line) for line in lines]
                self.heights = [
                    json.loads(line)
                    for line in (Path(tmp) / "heights.jsonl").read_text().splitlines()
                ]
                meta = json.loads((Path(tmp) / "load-meta.json").read_text())
                self.steps = json.loads((Path(tmp) / "steps.json").read_text())
        finally:
            node.stop.set()
            node.shutdown()
            for peer in self.peers:
                peer.shutdown()
            for thread in threads:
                thread.join()
            node.server_close()
            for peer in self.peers:
                peer.server_close()
        return start, node, txs, meta

    def test_submits_at_the_offered_rate(self):
        # 1000 byte txs at 0.02 MB/s: one every 50 ms, independent of inclusion.
        _, node, txs, meta = self.run_load(
            True, 1.0, rate=0.02, cap_txs=100, tx_timeout_s=5
        )
        self.assertIn(len(node.submits), range(19, 22))
        gaps = [b - a for a, b in zip(node.submits, node.submits[1:])]
        self.assertAlmostEqual(sorted(gaps)[len(gaps) // 2], 0.05, delta=0.01)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))
        self.assertEqual(meta["cap_waits"], 0)

    def test_round_robin_over_submit_nodes(self):
        _, _, txs, _ = self.run_load(
            True, 0.5, nodes=3, submit_nodes=2, rate=0.02, tx_timeout_s=5
        )
        self.assertEqual([tx["node"] for tx in txs[:4]], [0, 1, 0, 1])
        self.assertTrue(all(tx["target_view"] is None for tx in txs))

    def test_leader_mode_submits_to_the_upcoming_leader(self):
        # Three nodes, a view per 50 ms block, node `view % 3` leading: every submit goes to
        # the node leading two views past the one read, and lands on that node.
        _, node, txs, meta = self.run_load(
            True,
            1.0,
            peers=2,
            submit_nodes=3,
            submit_to="leader",
            leader_ahead=2,
            rate=0.1,
            tx_timeout_s=5,
        )
        self.assertGreater(len(txs), 50)
        self.assertEqual(meta["leader_fallbacks"], 0)
        self.assertGreater(meta["leader_polls"], 10)
        for tx in txs:
            self.assertIsNotNone(tx["target_view"])
            self.assertEqual(tx["node"], tx["target_view"] % 3)
        by_node = [node, *self.peers]
        for index, server in enumerate(by_node):
            self.assertEqual(
                len(server.submits), sum(1 for tx in txs if tx["node"] == index)
            )
        self.assertTrue(all(tx["status"] == "included" for tx in txs))

    def test_leader_mode_falls_back_when_no_submit_node_leads(self):
        # Only the first of three nodes takes submits: views the others lead go to it
        # round-robin, with no target view.
        _, _, txs, meta = self.run_load(
            True,
            1.0,
            peers=2,
            submit_nodes=1,
            submit_to="leader",
            leader_ahead=2,
            rate=0.1,
            tx_timeout_s=5,
        )
        self.assertGreater(meta["leader_fallbacks"], 0)
        self.assertTrue(all(tx["node"] == 0 for tx in txs))
        targeted = [tx for tx in txs if tx["target_view"] is not None]
        self.assertGreater(len(targeted), 0)
        self.assertLess(len(targeted), len(txs))
        self.assertTrue(all(tx["target_view"] % 3 == 0 for tx in targeted))

    def test_cap_blocks_until_timeout(self):
        _, node, txs, meta = self.run_load(
            False, 2.5, rate=1.0, cap_txs=4, tx_timeout_s=1
        )
        # Room under the cap frees only on timeout, the first 1 s after the first submit.
        self.assertGreater(node.submits[4] - node.submits[0], 0.9)
        self.assertGreater(len(node.submits), 4)
        self.assertEqual(meta["max_in_flight"], 4)
        self.assertGreater(meta["cap_waits"], 0)
        self.assertTrue(all(tx["status"] == "timeout" for tx in txs))

    def test_latency_excludes_queued_submits(self):
        # One submit thread and 0.2 s per submit: submits queue up behind each other, and
        # neither the queue nor the tracker waiting behind it counts as latency.
        _, _, txs, _ = self.run_load(
            True,
            1.0,
            accept_delay=0.2,
            workers=1,
            rate=0.02,
            cap_txs=100,
            tx_timeout_s=5,
        )
        self.assertGreater(len(txs), 5)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))
        latencies = [tx["t_included"] - tx["t_submit"] for tx in txs]
        self.assertLess(max(latencies), 1.0)

    def test_queued_submit_does_not_time_out(self):
        # Submits wait up to 1.5 s in the one thread's queue, longer than tx_timeout_s.
        _, _, txs, meta = self.run_load(
            True,
            2.0,
            accept_delay=0.3,
            workers=1,
            rate=0.02,
            cap_txs=5,
            tx_timeout_s=1,
        )
        self.assertGreater(len(txs), 5)
        self.assertEqual({tx["status"] for tx in txs}, {"included"})
        self.assertLessEqual(meta["max_in_flight"], 5)

    def test_slow_payloads_do_not_delay_block_times(self):
        # Payload scans fall behind a block every 50 ms; block times must not.
        _, _, txs, _ = self.run_load(
            True,
            0.5,
            payload_delay=0.1,
            rate=0.02,
            cap_txs=100,
            tx_timeout_s=10,
        )
        self.assertGreater(len(txs), 5)
        self.assertEqual({tx["status"] for tx in txs}, {"included"})
        self.assertLess(max(tx["t_included"] - tx["t_submit"] for tx in txs), 0.4)

    def test_heights_on_validators_and_query_node(self):
        _, _, txs, _ = self.run_load(
            True, 0.5, query_lag=0.3, rate=0.02, cap_txs=100, tx_timeout_s=5
        )
        self.assertEqual({tx["status"] for tx in txs}, {"included"})
        by_height = {h["height"]: h for h in self.heights}
        for tx in txs:
            h = by_height[tx["height"]]
            self.assertEqual(tx["t_included"], h["query"])
            self.assertGreaterEqual(h["scanned"], h["query"])
        lags = [h["query"] - h["validator"] for h in self.heights if h["query"]]
        self.assertAlmostEqual(statistics.median(lags), 0.3, delta=0.12)

    def test_included_before_the_submit_returns(self):
        _, _, txs, _ = self.run_load(
            True, 0.6, reply_delay=0.4, rate=0.005, cap_txs=100, tx_timeout_s=2
        )
        self.assertGreater(len(txs), 2)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))

    def test_lost_payload_does_not_stall_later_blocks(self):
        with (
            mock.patch.object(bench, "MISSING_PAYLOAD_S", 1.0),
            self.assertLogs(bench.log, "WARNING"),
        ):
            _, _, txs, meta = self.run_load(
                True,
                2.0,
                lost={10},
                rate=0.02,
                cap_txs=8,
                tx_timeout_s=1.5,
            )
        self.assertEqual(meta["missing_payloads"], [10])
        self.assertEqual(meta["cap_waits"], 0)
        included = [tx for tx in txs if tx["status"] == "included"]
        self.assertLessEqual(len(txs) - len(included), 2)
        self.assertLess(max(tx["t_included"] - tx["t_submit"] for tx in included), 0.5)

    def test_late_payload_counts_from_its_block(self):
        _, _, txs, meta = self.run_load(
            True,
            1.5,
            late={10: 1.0},
            rate=0.02,
            cap_txs=8,
            tx_timeout_s=3,
        )
        self.assertEqual(meta["missing_payloads"], [])
        self.assertEqual(meta["cap_waits"], 0)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))
        self.assertLess(max(tx["t_included"] - tx["t_submit"] for tx in txs), 0.7)

    def test_staircase_stops_at_the_first_failing_step_and_refines(self):
        # 4 txs of 1000 bytes per 50 ms block: 0.08 MB/s of capacity. 2 s steps: a shorter
        # measured half holds too few transactions for a steady decided rate.
        with mock.patch.object(bench, "COUNTER_POLL_S", 0.05):
            self.run_load(
                True, 2.0, block_txs=4, steps=(0.02, 0.04, 0.16), tx_timeout_s=1
            )
        self.assertEqual(
            [
                (
                    s["rate_mb_s"],
                    s["refine"],
                    not s["consensus_fails"] + s["query_fails"],
                )
                for s in self.steps
            ],
            [
                (0.02, False, True),
                (0.04, False, True),
                (0.16, False, False),
                (0.1, True, False),
            ],
        )

    def test_refine_starts_after_the_backlog_drained(self):
        # 80 tx/s of capacity: 0.1 MB/s leaves 40 txs behind; 0.07 MB/s alone keeps up but
        # drains that backlog only at 10 tx/s, adding latency over the 300 ms target.
        with mock.patch.object(bench, "COUNTER_POLL_S", 0.05):
            _, _, _, meta = self.run_load(
                True,
                2.0,
                block_txs=4,
                steps=(0.04, 0.1),
                latency_target_ms=300,
                tx_timeout_s=10,
            )
        verdicts = [
            (s["rate_mb_s"], s["refine"], not s["consensus_fails"] + s["query_fails"])
            for s in self.steps
        ]
        self.assertEqual(
            verdicts,
            [
                (0.04, False, True),
                (0.1, False, False),
                (0.07000000000000001, True, True),
            ],
        )
        self.assertGreater(meta["drain_s"], 0.2)
        self.assertFalse(meta["refine_skipped"])

    def test_refine_is_skipped_when_the_backlog_does_not_drain(self):
        with (
            mock.patch.object(bench, "COUNTER_POLL_S", 0.05),
            mock.patch.object(bench, "drain", mock.AsyncMock(return_value=None)),
            self.assertLogs(bench.log, "WARNING"),
        ):
            _, _, _, meta = self.run_load(
                True, 1.0, block_txs=4, steps=(0.02, 0.16), tx_timeout_s=1
            )
        self.assertEqual([s["rate_mb_s"] for s in self.steps], [0.02, 0.16])
        self.assertTrue(meta["refine_skipped"])

    def test_step_ends_on_time_while_waiting_for_room(self):
        # Nothing is included: the cap fills at once and frees only on timeouts after 3 s.
        self.run_load(False, 1.0, rate=0.02, cap_txs=2, tx_timeout_s=3)
        (step,) = self.steps
        self.assertLess(step["t_end"] - step["t_start"], 1.3)

    def test_lost_payload_is_skipped(self):
        with (
            mock.patch.object(bench, "MISSING_PAYLOAD_S", 0.2),
            self.assertLogs(bench.log, "WARNING"),
        ):
            _, _, txs, meta = self.run_load(
                True, 1.0, lost={3}, rate=0.02, tx_timeout_s=1
            )
        self.assertEqual(meta["missing_payloads"], [3])
        self.assertGreater(sum(tx["status"] == "included" for tx in txs), 4)


def write_run_dir(out):
    """Steps at 1 MB/s (t 100 to 130) and 2 MB/s (130 to 160). Every node decides 1 MB/s in 2
    blocks/s and 4 views/s and uses 0.5 cores, the host is half busy, and a 1 MB transaction
    goes out every second from 110 to 150 (one times out): its block shows on a validator
    after 0.5 s and on node0 after 0.8 s, and is scanned 1.1 s after that."""
    t0, t1 = 100.0, 160.0

    def jsonl(name, records):
        (out / name).write_text("".join(json.dumps(r) + "\n" for r in records))

    metrics = {
        "consensus_finalized_bytes_sum": 1e6,
        "consensus_finalized_bytes_count": 2.0,
        "consensus_last_decided_view": 4.0,
        "consensus_last_synced_block_height": 2.0,
        "process_cpu_seconds_total": 0.5,
    }
    jsonl(
        "metrics.jsonl",
        (
            {
                "ts": ts,
                "node": node,
                "ok": True,
                "m": {k: v * ts for k, v in metrics.items()},
            }
            for ts in range(90, 175, 5)
            for node in bench.NODES
        ),
    )
    jsonl(
        "host.jsonl",
        (
            {
                "ts": ts,
                "cpu": [50 * ts, 0, 0, 50 * ts, 0, 0, 0, 0, 0, 0],
                "mem_avail": 1000 + ts,
                "procs": {"node0": {"cpu_s": 0.5 * ts, "rss": 10 * ts}},
            }
            for ts in range(90, 172, 2)
        ),
    )
    txs: list[dict] = [
        {
            "id": i,
            "node": 0,
            "t_submit": t,
            "t_included": t + 0.8,
            "height": 1000 + i,
            "status": "included",
        }
        for i, t in enumerate(range(110, 151))
    ]
    txs.append(
        {
            "id": 99,
            "node": 0,
            "t_submit": 120.5,
            "t_included": None,
            "height": None,
            "status": "timeout",
        }
    )
    jsonl("load.jsonl", txs)
    heights = [
        {
            "height": tx["height"],
            "validator": tx["t_submit"] + 0.5,
            "query": tx["t_included"],
            "scanned": tx["t_included"] + 1.1,
        }
        for tx in txs[:-1]
    ]
    heights.append(
        {"height": 2000, "validator": 130.0, "query": 130.5, "scanned": None}
    )
    jsonl("heights.jsonl", heights)
    counters = [
        {"ts": float(ts), "decided_bytes": 1e6 * ts, "timeouts": 0}
        for ts in range(100, 161)
    ]
    jsonl("consensus.jsonl", counters)
    steps = [
        {
            "rate_mb_s": 1.0,
            "refine": False,
            "t_start": 100.0,
            "t_mid": 115.0,
            "t_end": 130.0,
        },
        {
            "rate_mb_s": 2.0,
            "refine": False,
            "t_start": 130.0,
            "t_mid": 145.0,
            "t_end": 160.0,
        },
    ]
    calib = {"sha256_1t_mb_s": 2000.0, "sha256_mt_mb_s": 8000.0, "fsync_per_s": 300.0}
    files = {
        "load-meta.json": {
            "submit_errors": 0,
            "max_in_flight": 4,
            "cap_waits": 0,
            "missing_payloads": [],
            "drain_s": None,
            "refine_skipped": False,
        },
        "calibration.json": {"before": calib, "after": calib},
        "steps.json": [
            bench.judge_step(s, bench.BenchConfig(), txs, heights, counters, s["t_end"])
            for s in steps
        ],
        "sysinfo.json": {"runner": make_result()["runner"]},
        "stake-table.json": {
            "stake_table": [{"stake_table_entry": {"stake_amount": "0x1"}}] * 3
        },
        "run.json": {
            "t0": t0,
            "t1": t1,
            "started": 0.0,
            "wall_s": 300.0,
            "ready_s": 40.0,
            "teardown": [],
            "config_hash": "abc",
            "meta": make_result()["run"],
        },
    }
    for name, data in files.items():
        (out / name).write_text(json.dumps(data))


class AnalyzeTest(unittest.TestCase):
    def test_steps_and_capacity(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_run_dir(Path(tmp))
            result = bench.analyze(Path(tmp), bench.BenchConfig())
        one, two = result["steps"]
        self.assertAlmostEqual(one["decided_mb_s"], 1.0)
        self.assertEqual(one["timeouts"], 0)
        self.assertAlmostEqual(one["consensus_latency_ms"]["p50"], 500.0)
        self.assertAlmostEqual(one["query_lag_ms"]["p50"], 300.0)
        self.assertAlmostEqual(one["latency_ms"]["p50"], 800.0)
        self.assertAlmostEqual(one["mean_view_ms"], 250.0)
        self.assertAlmostEqual(one["cpu_s_per_mb"], 1.5)
        self.assertEqual(one["node_cpu"], {"node0": 0.5, "node1": 0.5, "node2": 0.5})
        self.assertIsNone(one["postgres_cpu"])
        self.assertTrue(one["passed"])
        self.assertEqual(two["consensus_fails"], ["decided 50% of offered"])
        self.assertEqual(result["capacity"]["overall"], {"mb_s": 1.0, "bounded": True})
        node0, load = result["nodes"]["node0"], result["load"]
        self.assertEqual((node0["decided_blocks"], node0["cpu_cores"]), (120, 0.5))
        self.assertEqual(
            (result["window"]["height_start"], result["window"]["height_end"]),
            (200, 320),
        )
        self.assertAlmostEqual(result["host"]["util_mean"], 0.5)
        self.assertAlmostEqual(result["processes"]["node0"]["cpu_cores_mean"], 0.5)
        self.assertEqual(
            (load["submitted"], load["included"], load["timeouts"]), (42, 41, 1)
        )
        self.assertAlmostEqual(load["tracker_lag_ms"]["p99"], 100.0)
        self.assertEqual(
            result["validity"], {"valid": True, "noisy": False, "reasons": []}
        )

    def test_result_json_is_finite(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_run_dir(Path(tmp))
            result = bench.analyze(Path(tmp), bench.BenchConfig())
            bench.write_json(Path(tmp) / "result.json", result)
            with self.assertRaises(ValueError):
                bench.write_json(Path(tmp) / "bad.json", {"x": math.inf})

    def test_report_keeps_the_ramp_verdict(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_run_dir(Path(tmp))
            path = Path(tmp) / "steps.json"
            steps = json.loads(path.read_text())
            steps[0]["consensus_fails"] = ["decided 10% of offered"]
            path.write_text(json.dumps(steps))
            result = bench.analyze(Path(tmp), bench.BenchConfig())
        self.assertFalse(result["steps"][0]["passed"])
        self.assertEqual(result["capacity"]["overall"], {"mb_s": None, "bounded": True})

    def test_no_scrapes_is_invalid_not_a_crash(self):
        cfg = bench.BenchConfig()
        with tempfile.TemporaryDirectory() as tmp:
            write_run_dir(Path(tmp))
            (Path(tmp) / "metrics.jsonl").write_text(
                "".join(
                    json.dumps({"ts": ts, "node": node, "ok": False}) + "\n"
                    for ts in range(90, 175, 5)
                    for node in bench.NODES
                )
            )
            result = bench.analyze(Path(tmp), cfg)
        self.assertFalse(result["validity"]["valid"])
        self.assertIsNone(result["steps"][0]["mean_view_ms"])
        self.assertIn(
            "node0 metrics answered for only 0%", result["validity"]["reasons"][0]
        )
        self.assertIn("run **invalid**", bench.render(result, None))


class StaircaseTest(unittest.TestCase):
    def test_ramp_until_the_first_failure_then_refine(self):
        ramp = (4.0, 6.0, 8.0)
        self.assertEqual(bench.next_rate(ramp, []), 4.0)
        self.assertEqual(bench.next_rate(ramp, [True]), 6.0)
        self.assertEqual(bench.next_rate(ramp, [True, False]), 5.0)
        self.assertIsNone(bench.next_rate(ramp, [True, False, True]))
        self.assertIsNone(bench.next_rate(ramp, [True, True, True]))

    def test_first_step_failing_ends_the_ramp(self):
        self.assertIsNone(bench.next_rate((4.0, 6.0), [False]))

    def test_drain_waits_for_the_query_node(self):
        state = types.SimpleNamespace(pending={})
        counters = [{"decided_bytes": 1}, {"decided_bytes": 1}]
        heights = bench.Heights(0)
        heights.saw("validator", 5, 0.0)
        heights.saw("query", 3, 0.0)
        self.assertIsNone(asyncio.run(bench.drain(state, counters, heights, 0.3)))
        heights.saw("query", 5, 0.0)
        self.assertIsNotNone(asyncio.run(bench.drain(state, counters, heights, 0.3)))

    def test_drain_does_not_chase_new_validator_heights(self):
        state = types.SimpleNamespace(pending={})
        counters = [{"decided_bytes": 1}, {"decided_bytes": 1}]
        heights = bench.Heights(0)
        heights.saw("validator", 5, 0.0)

        async def run() -> float | None:
            task = asyncio.create_task(bench.drain(state, counters, heights, 1.0))
            await asyncio.sleep(0.05)
            heights.saw("validator", 6, 0.0)
            heights.saw("query", 5, 0.0)
            return await task

        self.assertIsNotNone(asyncio.run(run()))

    def test_theil_sen_ignores_an_outlier(self):
        points = [(float(x), 2.0 * x) for x in range(10)] + [(10.0, 100.0)]
        self.assertAlmostEqual(bench.theil_sen(points), 2.0)
        self.assertIsNone(bench.theil_sen([(1.0, 1.0)]))


def step_window(rate=10.0, start=0.0):
    return {
        "rate_mb_s": rate,
        "refine": False,
        "t_start": start,
        "t_mid": start + 15,
        "t_end": start + 30,
    }


class StepMeasuresTest(unittest.TestCase):
    """A 10 MB/s step, t 0 to 30, measured from 15: one 1 MB tx every 0.1 s."""

    def measures(
        self,
        decided_mb_s=10.0,
        timeouts=0,
        consensus_s=0.5,
        query_s=0.2,
        query_growth=0.0,
        now=math.inf,
    ):
        cfg = bench.BenchConfig()
        txs = [
            {"t_submit": i / 10, "height": i, "status": "included"} for i in range(300)
        ]
        heights = [
            {
                "height": i,
                "validator": i / 10 + consensus_s,
                "query": i / 10 + consensus_s + query_s + query_growth * i / 10,
                "scanned": None,
            }
            for i in range(300)
        ]
        counters = [
            {
                "ts": float(t),
                "decided_bytes": decided_mb_s * 1e6 * t,
                "timeouts": timeouts * (t > 20),
            }
            for t in range(31)
        ]
        return bench.step_measures(step_window(), cfg, txs, heights, counters, now)

    def test_keeping_up(self):
        m = self.measures()
        self.assertAlmostEqual(m["decided_mb_s"], 10.0)
        self.assertEqual(m["timeouts"], 0)
        self.assertAlmostEqual(m["consensus_latency_ms"]["p50"], 500.0)
        self.assertAlmostEqual(m["query_lag_ms"]["p50"], 200.0)
        self.assertAlmostEqual(m["query_lag_slope_ms_s"], 0.0)
        self.assertEqual(bench.step_fails(m, 10.0, bench.BenchConfig()), ([], []))

    def test_consensus_rules(self):
        m = self.measures(decided_mb_s=9.0, timeouts=1, consensus_s=1.5)
        consensus, query = bench.step_fails(m, 10.0, bench.BenchConfig())
        self.assertEqual(query, [])
        self.assertEqual(
            consensus,
            [
                "decided 90% of offered",
                "1 view timeouts",
                "consensus latency p50 1500 ms > 1000 ms",
            ],
        )

    def test_pending_transactions_count_once_over_target(self):
        cfg = bench.BenchConfig()
        txs = [
            {"t_submit": 16.0 + i / 10, "height": None, "status": "pending"}
            for i in range(20)
        ]
        m = bench.step_measures(step_window(), cfg, txs, [], [], 20.0)
        # By t 20 all 20 are over the 1000 ms target; by t 18 the 10 submitted before 17.
        self.assertEqual(m["consensus_latency_ms"]["n"], 20)
        m = bench.step_measures(step_window(), cfg, txs, [], [], 18.0)
        self.assertEqual(m["consensus_latency_ms"]["n"], 10)
        self.assertGreater(m["consensus_latency_ms"]["p50"], 1000.0)

    def test_decided_rate_is_not_quantized_by_blocks(self):
        # A 20 MB block every 2 s at t 1, 3, 5, ...: 10 MB/s, but the counter samples at 15
        # and 30 see 7 blocks in 15 s.
        counters = [
            {"ts": float(t), "decided_bytes": 20e6 * ((t + 1) // 2), "timeouts": 0}
            for t in range(31)
        ]
        m = bench.step_measures(
            step_window(), bench.BenchConfig(), [], [], counters, 30.0
        )
        self.assertAlmostEqual(m["decided_mb_s"], 10.0, delta=0.3)
        self.assertEqual(bench.step_fails(m, 10.0, bench.BenchConfig()), ([], []))

    def test_growing_query_lag(self):
        m = self.measures(query_growth=0.2)
        self.assertAlmostEqual(m["query_lag_slope_ms_s"], 200.0, delta=1)
        consensus, query = bench.step_fails(m, 10.0, bench.BenchConfig())
        self.assertEqual(consensus, [])
        self.assertEqual(query[0], "query lag grows 200 ms/s")

    def test_heights_not_yet_on_the_query_node_count_as_lagging(self):
        # At t 22 nothing is on the query node yet: heights older than the target lag.
        m = self.measures(query_s=100.0, now=22.0)
        self.assertGreater(m["query_lag_ms"]["p50"], 1000.0)
        _, query = bench.step_fails(m, 10.0, bench.BenchConfig())
        self.assertIn("query lag p50", query[0])


def verdict(rate, consensus=(), query=()):
    return {
        "rate_mb_s": rate,
        "consensus_fails": list(consensus),
        "query_fails": list(query),
    }


class CapacityTest(unittest.TestCase):
    def test_all_pass_is_a_lower_bound(self):
        cap = bench.capacity([verdict(4.0), verdict(6.0)])
        self.assertEqual(cap["overall"], {"mb_s": 6.0, "bounded": False})
        self.assertIsNone(cap["fail_rule"])

    def test_consensus_limit(self):
        cap = bench.capacity(
            [
                verdict(4.0),
                verdict(6.0),
                verdict(8.0, consensus=["decided 90% of offered"]),
                verdict(7.0),
            ]
        )
        self.assertEqual(cap["overall"], {"mb_s": 7.0, "bounded": True})
        self.assertEqual(cap["consensus"], {"mb_s": 7.0, "bounded": True})
        self.assertEqual(cap["query_node"], {"mb_s": 8.0, "bounded": False})
        self.assertEqual(cap["fail_rule"], "decided 90% of offered at 8 MB/s")
        self.assertEqual(
            bench.capacity_line(cap),
            "Capacity **7 MB/s**: consensus limits (decided 90% of offered at 8 MB/s).",
        )

    def test_query_node_limit(self):
        cap = bench.capacity(
            [
                verdict(4.0),
                verdict(6.0, query=["query lag p50 1500 ms > 1000 ms"]),
                verdict(5.0),
            ]
        )
        self.assertEqual(cap["query_node"], {"mb_s": 5.0, "bounded": True})
        self.assertEqual(cap["consensus"], {"mb_s": 6.0, "bounded": False})
        self.assertEqual(
            bench.capacity_line(cap),
            "Capacity **5 MB/s**: query node limits at 5 MB/s, consensus >= 6 MB/s "
            "(query lag p50 1500 ms > 1000 ms at 6 MB/s).",
        )

    def test_first_step_fails(self):
        cap = bench.capacity([verdict(4.0, consensus=["1 view timeouts"])])
        self.assertEqual(cap["overall"], {"mb_s": None, "bounded": True})
        self.assertEqual(
            bench.capacity_line(cap),
            "Capacity **< 4 MB/s**: consensus limits (1 view timeouts at 4 MB/s).",
        )

    def test_no_failure(self):
        self.assertEqual(
            bench.capacity_line(bench.capacity([verdict(4.0), verdict(6.0)])),
            "Capacity **>= 6 MB/s**: no step failed.",
        )


class PaceTest(unittest.TestCase):
    def test_interval_from_rate(self):
        self.assertAlmostEqual(bench.tx_interval_s(1_000_000, 20.0), 0.05)

    def test_cap_is_seconds_of_load(self):
        self.assertEqual(bench.step_cap(bench.BenchConfig(), 7.0), 35)

    def test_on_time_keeps_the_schedule(self):
        self.assertEqual(bench.next_due(10.0, 0.5, 10.2), 10.5)

    def test_late_restarts_from_now(self):
        self.assertEqual(bench.next_due(10.0, 0.5, 12.0), 12.0)


class HistogramTest(unittest.TestCase):
    def test_interpolates_inside_bucket(self):
        m0 = {}
        m1 = {
            'op_bucket{le="0.1"}': 50.0,
            'op_bucket{le="0.2"}': 100.0,
            'op_bucket{le="+Inf"}': 100.0,
            "op_count": 100.0,
            "op_sum": 10.0,
        }
        q = bench.histogram_quantiles([(m0, m1)], "op")
        self.assertAlmostEqual(q["p50"], 100.0)
        self.assertAlmostEqual(q["p99"], 198.0)
        self.assertAlmostEqual(q["mean"], 100.0)


class InfBucketTest(unittest.TestCase):
    def test_quantile_in_inf_bucket_is_the_top_finite_bound(self):
        m1 = {
            'op_bucket{le="0.1"}': 50.0,
            'op_bucket{le="+Inf"}': 100.0,
            "op_count": 100.0,
            "op_sum": 30.0,
        }
        q = bench.histogram_quantiles([({}, m1)], "op")
        self.assertAlmostEqual(q["p50"], 100.0)
        self.assertAlmostEqual(q["p99"], 100.0)
        self.assertAlmostEqual(q["max"], 100.0)


class ClosingHandler(BaseHTTPRequestHandler):
    """Answers keep-alive style, then closes the connection without saying so."""

    protocol_version = "HTTP/1.1"
    connections = 0

    def setup(self):
        super().setup()
        type(self).connections += 1

    def log_message(self, format, *args):
        pass

    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Length", "2")
        self.end_headers()
        self.wfile.write(b"42")
        self.close_connection = True


class HttpPoolTest(unittest.TestCase):
    def test_stale_connection_is_retried_on_a_fresh_one(self):
        server = ThreadingHTTPServer(("127.0.0.1", 0), ClosingHandler)
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        url = f"http://127.0.0.1:{server.server_address[1]}/x"
        pool = bench.HttpPool()
        try:
            self.assertEqual(pool.request("GET", url), (200, b"42"))
            time.sleep(0.1)
            self.assertEqual(pool.request("GET", url), (200, b"42"))
        finally:
            pool.close()
            server.shutdown()
            thread.join()
            server.server_close()
        self.assertEqual(ClosingHandler.connections, 2)


class WindowTest(unittest.TestCase):
    def test_no_samples_gives_zero_heights(self):
        series = {node: [] for node in bench.NODES}
        self.assertEqual(
            bench.window(series, 1.0, 2.0),
            {"t0": 1.0, "t1": 2.0, "height_start": 0, "height_end": 0},
        )



class BlockMeasuresTest(unittest.TestCase):
    def test_transactions_per_block_and_gaps_in_the_window(self):
        heights = [
            {"height": 10, "validator": 9.0, "query": None, "scanned": None},
            {"height": 11, "validator": 10.0, "query": None, "scanned": None},
            {"height": 12, "validator": 10.1, "query": None, "scanned": None},
            # Seen in the same poll as 12: a zero gap.
            {"height": 13, "validator": 10.1, "query": None, "scanned": None},
            {"height": 14, "validator": 10.6, "query": None, "scanned": None},
            {"height": 15, "validator": None, "query": None, "scanned": None},
        ]
        txs = [{"height": h} for h in (11, 11, 11, 12, 14, 14, 10, None)]
        block_txs, gaps = bench.block_measures(txs, heights, 10.0, 20.0)
        # Heights 11..14 carry 3, 1, 0 and 2 of the bench's transactions.
        self.assertEqual(block_txs["n"], 4)
        self.assertEqual(block_txs["mean"], 1.5)
        self.assertEqual(block_txs["max"], 3.0)
        self.assertEqual(gaps["n"], 3)
        self.assertAlmostEqual(gaps["p50"], 100.0)
        self.assertAlmostEqual(gaps["max"], 500.0)

    def test_none_without_blocks_in_the_window(self):
        self.assertEqual(bench.block_measures([], [], 0.0, 1.0), (None, None))


class LoadLinesTest(unittest.TestCase):
    def test_round_robin_config_reads_as_before(self):
        text = "\n".join(bench.load_lines(make_result()))
        self.assertIn("txs to 3 nodes", text)
        self.assertNotIn("fanout", text)
        self.assertNotIn("leader reads", text)

    def test_leader_config_names_the_target_and_node_settings(self):
        result = make_result()
        result["config"] |= {
            "submit_to": "leader",
            "leader_ahead": 3,
            "fanout": 0,
            "empty_block_delay_ms": 100,
        }
        result["load"] |= {"leader_polls": 1200, "leader_fallbacks": 2}
        text = "\n".join(bench.load_lines(result))
        self.assertIn(
            "to the leader 3 views ahead of node1's view, read every 20 ms, among 3 nodes",
            text,
        )
        self.assertIn("transaction fanout 0, empty block delay 100 ms", text)
        self.assertIn("1200 leader reads, 2 submits fell back to round-robin", text)


class RunArgsTest(unittest.TestCase):
    def test_submit_mode_and_node_settings(self):
        args = bench.parse_args(
            [
                "run",
                "--bin-dir",
                "x",
                "--submit-to",
                "leader",
                "--fanout",
                "0",
                "--empty-block-delay-ms",
                "100",
            ]
        )
        cfg = bench.config_from_args(args)
        self.assertEqual(
            (cfg.submit_to, cfg.leader_ahead, cfg.fanout, cfg.empty_block_delay_ms),
            ("leader", 2, 0, 100),
        )

    def test_leader_poll_below_a_millisecond_is_refused(self):
        with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
            bench.parse_args(["run", "--bin-dir", "x", "--leader-poll-ms", "0"])

    def test_leader_ahead_beyond_the_window_is_refused(self):
        with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
            bench.parse_args(["run", "--bin-dir", "x", "--leader-ahead", "9"])

    def test_unknown_submit_mode_is_refused(self):
        with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
            bench.parse_args(["run", "--bin-dir", "x", "--submit-to", "random"])

if __name__ == "__main__":
    unittest.main()
