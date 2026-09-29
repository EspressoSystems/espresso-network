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
import json
import socket
import statistics
import subprocess
import sys
import tempfile
import threading
import time
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


def stat(value, unit, better):
    return {
        "value": value,
        "min": value * 0.9,
        "max": value * 1.1,
        "cv": 0.05,
        "unit": unit,
        "better": better,
    }


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


def make_result(mb_per_s=4.0, steal=0.0, config_hash="abc123"):
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
            "max_pending": 48,
            "tx_size": 100_000,
            "rate_mb_s": 4.0,
            "submit_nodes": 3,
            "subwindow_s": 30,
        },
        "config_hash": config_hash,
        "window": {"t0": 0.0, "t1": 180.0, "height_start": 100, "height_end": 500},
        "network": {
            "decided_mb_per_s": stat(mb_per_s, "MB/s", "higher"),
            "blocks_per_s": stat(2.0, "1/s", "higher"),
            "views_per_s": stat(2.0, "1/s", "higher"),
            "mean_view_ms": stat(500.0, "ms", "lower"),
            "timeouts": 0,
            "empty_block_frac": 0.1,
            "proposal_to_decide_ms": quantiles(900.0, 2000.0),
            "cpu_s_per_mb": stat(0.5, "s/MB", "lower"),
            "block_bytes_nonempty_mean": 2_000_000.0,
            "max_block_bytes": 100_000_000,
        },
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
            "submitted_per_s": 40.0,
            "included_per_s": 40.0,
            "inclusion_ratio": 1.0,
            "timeouts": 0,
            "submit_errors": 0,
            "max_in_flight": 48,
            "at_cap_frac": 0.8,
            "cap_waits": 0,
            "offered_mb_s": 4.0,
            "missing_payloads": [],
            "latency_ms": quantiles(1200.0, 2500.0),
            "consensus_latency_ms": quantiles(900.0, 2000.0),
            "query_lag_ms": quantiles(200.0, 400.0),
            "query_lag_end_ms": 200.0,
            "tracker_lag_ms": quantiles(50.0, 100.0),
            "tracker_lag_end_ms": 50.0,
            "in_flight_mean": 20.0,
        },
        "bound": {"kind": "keeping-up", "reason": "r"},
        "stake_table": ["0x1", "0x1", "0x1"],
        "validity": {"valid": True, "noisy": False, "reasons": []},
    }
    result["validity"] = bench.check_validity(result, {n: 1.0 for n in bench.NODES})
    return result


def compare(current, runs, error=None, source="main"):
    return bench.compare(current, {"runs": runs, "error": error, "source": source})


def row(comparison, label):
    return next(r for r in comparison["rows"] if r["label"] == label)


class RenderTest(unittest.TestCase):
    def test_summary_has_headline_and_delta(self):
        baseline = [make_result(mb_per_s=4.0) for _ in range(5)]
        current = make_result(mb_per_s=3.0)
        summary = bench.render(current, compare(current, baseline))
        self.assertIn("| main median (n=5) |", summary)
        self.assertRegex(
            summary,
            r"\| decided payload throughput \| 3 MB/s \|.*\| -25\.0% \| \*\*worse\*\*",
        )
        self.assertIn("Test CPU", summary)

    def test_verdict_first_detail_last(self):
        current = make_result()
        current["bound"] = {"kind": "behind", "reason": "75% of blocks empty"}
        summary = bench.render(current, None)
        order = [
            "**Behind the offered load** (75% of blocks empty)",
            "Baseline: none given",
            "| metric | this run |",
            "### Load",
            "<details>",
        ]
        self.assertEqual(sorted(order, key=summary.index), order)
        self.assertNotIn("| delta |", summary)

    def test_latency_breakdown(self):
        summary = bench.render(make_result(), None)
        for row in (
            "| consensus latency p50 | 900 ms |",
            "| consensus latency p99 | 2000 ms |",
            "| query lag p50 | 200 ms |",
            "| query lag p99 | 400 ms |",
            "| tx latency p50 (via query node) | 1200 ms |",
        ):
            self.assertIn(row, summary)
        self.assertIn("- benchmark tracker lag: p50 50, p95 100, p99 100", summary)

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
        current = make_result(mb_per_s=3.8)
        comparison = compare(current, [make_result(mb_per_s=4.0)])
        self.assertEqual(
            row(comparison, "decided payload throughput")["verdict"], "same"
        )

    def test_spread_widens_threshold(self):
        history = [make_result(mb_per_s=v) for v in (3.0, 4.0, 5.0, 3.5, 4.5)]
        verdict = row(
            compare(make_result(mb_per_s=3.0), history), "decided payload throughput"
        )
        self.assertGreater(verdict["threshold_pct"], 10)
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
        current = make_result(mb_per_s=1.0, steal=8.0)
        comparison = compare(current, [make_result()])
        self.assertEqual(
            row(comparison, "decided payload throughput")["verdict"], "inconclusive"
        )

    def test_proposal_to_decide_is_not_compared(self):
        current = make_result()
        keys = [r["key"] for r in compare(current, [make_result()])["rows"]]
        self.assertNotIn("network.proposal_to_decide_ms.p99", keys)
        self.assertNotIn("proposal to decide", bench.render(current, None))

    def test_timeouts_from_zero_baseline(self):
        current = make_result()
        current["network"]["timeouts"] = 3
        self.assertEqual(
            row(compare(current, [make_result()]), "consensus timeouts")["verdict"],
            "worse",
        )


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

    def test_stalled_sub_window_is_invalid(self):
        result = make_result()
        result["network"]["blocks_per_s"]["min"] = 0.0
        validity = bench.check_validity(result, {n: 1.0 for n in bench.NODES})
        self.assertFalse(validity["valid"])
        self.assertEqual(validity["reasons"], ["a 30 s sub-window decided no blocks"])


class ReportOnlyTest(unittest.TestCase):
    def report(self, current):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "config.json").write_text(
                json.dumps(dataclasses.asdict(bench.BenchConfig()))
            )
            baseline = out / "baseline.json"
            baseline.write_text(json.dumps({"runs": [make_result(mb_per_s=8.0)] * 3}))
            with (
                mock.patch.object(bench, "analyze", return_value=current),
                mock.patch("builtins.print"),
                self.assertLogs(bench.log),
            ):
                code = bench.write_report(out, bench.load_baseline(baseline))
            return code, (out / "summary.md").read_text()

    def test_regression_still_exits_zero(self):
        code, summary = self.report(make_result(mb_per_s=2.0))
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
    The query API shows a block `query_lag` s after the validator status API."""

    def __init__(
        self,
        include,
        lost=frozenset(),
        accept_delay=0.0,
        reply_delay=0.0,
        late=None,
        payload_delay=0.0,
        query_lag=0.0,
    ):
        super().__init__(("127.0.0.1", 0), FakeHandler)
        self.query_lag = query_lag
        self.include = include
        self.lost = lost
        self.late = late or {}
        self.accept_delay = accept_delay
        self.reply_delay = reply_delay
        self.payload_delay = payload_delay
        self.lock = threading.Lock()
        self.pending = []
        self.blocks = [b""]
        self.made = [time.time()]
        self.submits = []
        self.max_outstanding = 0
        self.stop = threading.Event()

    @property
    def url(self):
        return f"http://127.0.0.1:{self.server_address[1]}"

    def produce(self):
        while not self.stop.wait(0.05):
            with self.lock:
                block = b"".join(self.pending) if self.include else b""
                if self.include:
                    self.pending = []
                self.blocks.append(block)
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
        data = json.dumps(body).encode()
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
        **cfg,
    ):
        node = FakeNode(
            include,
            lost=lost,
            accept_delay=accept_delay,
            reply_delay=reply_delay,
            late=late,
            payload_delay=payload_delay,
            query_lag=query_lag,
        )
        threads = [
            threading.Thread(target=node.serve_forever),
            threading.Thread(target=node.produce),
        ]
        for thread in threads:
            thread.start()
        config = bench.BenchConfig(**{"tx_size": 1000, "workers": 3} | cfg)
        try:
            with tempfile.TemporaryDirectory() as tmp:
                start = time.time()
                asyncio.run(
                    bench.generate_load(
                        config,
                        [node.url] * nodes,
                        node.url,
                        [node.url, node.url],
                        start + duration,
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
        finally:
            node.stop.set()
            node.shutdown()
            for thread in threads:
                thread.join()
            node.server_close()
        return start, node, txs, meta

    def test_submits_at_the_offered_rate(self):
        # 1000 byte txs at 0.02 MB/s: one every 50 ms, independent of inclusion.
        _, node, txs, meta = self.run_load(
            True, 1.0, rate_mb_s=0.02, max_pending=100, tx_timeout_s=5
        )
        self.assertIn(len(node.submits), range(19, 22))
        gaps = [b - a for a, b in zip(node.submits, node.submits[1:])]
        self.assertAlmostEqual(sorted(gaps)[len(gaps) // 2], 0.05, delta=0.01)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))
        self.assertEqual(meta["cap_waits"], 0)

    def test_round_robin_over_submit_nodes(self):
        _, _, txs, _ = self.run_load(
            True, 0.5, nodes=3, submit_nodes=2, rate_mb_s=0.02, tx_timeout_s=5
        )
        self.assertEqual([tx["node"] for tx in txs[:4]], [0, 1, 0, 1])

    def test_cap_blocks_until_timeout(self):
        _, node, txs, meta = self.run_load(
            False, 2.5, rate_mb_s=1.0, max_pending=4, tx_timeout_s=1
        )
        # A permit returns only on timeout, and the first one is 1 s after the first submit.
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
            rate_mb_s=0.02,
            max_pending=100,
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
            rate_mb_s=0.02,
            max_pending=5,
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
            rate_mb_s=0.02,
            max_pending=100,
            tx_timeout_s=10,
        )
        self.assertGreater(len(txs), 5)
        self.assertEqual({tx["status"] for tx in txs}, {"included"})
        self.assertLess(max(tx["t_included"] - tx["t_submit"] for tx in txs), 0.4)

    def test_heights_on_validators_and_query_node(self):
        _, _, txs, _ = self.run_load(
            True, 0.5, query_lag=0.3, rate_mb_s=0.02, max_pending=100, tx_timeout_s=5
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
            True, 0.6, reply_delay=0.4, rate_mb_s=0.005, max_pending=100, tx_timeout_s=2
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
                rate_mb_s=0.02,
                max_pending=8,
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
            rate_mb_s=0.02,
            max_pending=8,
            tx_timeout_s=3,
        )
        self.assertEqual(meta["missing_payloads"], [])
        self.assertEqual(meta["cap_waits"], 0)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))
        self.assertLess(max(tx["t_included"] - tx["t_submit"] for tx in txs), 0.7)

    def test_lost_payload_is_skipped(self):
        with (
            mock.patch.object(bench, "MISSING_PAYLOAD_S", 0.2),
            self.assertLogs(bench.log, "WARNING"),
        ):
            _, _, txs, meta = self.run_load(
                True, 1.0, lost={3}, rate_mb_s=0.02, tx_timeout_s=1
            )
        self.assertEqual(meta["missing_payloads"], [3])
        self.assertGreater(sum(tx["status"] == "included" for tx in txs), 4)


def write_run_dir(out):
    """A 60 s window (t 100 to 160) where every node decides 1 MB/s in 2 blocks/s and 4 views/s
    and uses 0.5 cores, the host is half busy, and 41 of 42 transactions land 2 s after submit:
    their block shows on a validator after 1.5 s and on node0 after 2 s, and is scanned 0.1 s
    later."""
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
            "t_included": t + 2.0,
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
            "validator": tx["t_submit"] + 1.5,
            "query": tx["t_included"],
            "scanned": tx["t_included"] + 0.1,
        }
        for tx in txs[:-1]
    ]
    heights.append(
        {"height": 2000, "validator": 130.0, "query": 130.5, "scanned": None}
    )
    jsonl("heights.jsonl", heights)
    calib = {"sha256_1t_mb_s": 2000.0, "sha256_mt_mb_s": 8000.0, "fsync_per_s": 300.0}
    files = {
        "load-meta.json": {
            "submit_errors": 0,
            "max_in_flight": 4,
            "cap_waits": 0,
            "missing_payloads": [],
        },
        "calibration.json": {"before": calib, "after": calib},
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
    def test_window_rates(self):
        cfg = bench.BenchConfig(measure_s=60, subwindow_s=20, rate_mb_s=1.0)
        with tempfile.TemporaryDirectory() as tmp:
            write_run_dir(Path(tmp))
            result = bench.analyze(Path(tmp), cfg)
        net, node0, load = result["network"], result["nodes"]["node0"], result["load"]
        self.assertAlmostEqual(net["decided_mb_per_s"]["value"], 1.0)
        self.assertAlmostEqual(net["blocks_per_s"]["min"], 2.0)
        self.assertAlmostEqual(net["mean_view_ms"]["value"], 250.0)
        self.assertAlmostEqual(net["cpu_s_per_mb"]["value"], 1.5)
        self.assertEqual(net["timeouts"], 0)
        self.assertEqual((node0["decided_blocks"], node0["cpu_cores"]), (120, 0.5))
        self.assertEqual(
            (result["window"]["height_start"], result["window"]["height_end"]),
            (200, 320),
        )
        self.assertAlmostEqual(result["host"]["util_mean"], 0.5)
        self.assertEqual(result["host"]["mem_avail_min_bytes"], 1100)
        self.assertAlmostEqual(result["processes"]["node0"]["cpu_cores_mean"], 0.5)
        self.assertAlmostEqual(load["submitted_per_s"], 42 / 60)
        self.assertAlmostEqual(load["included_per_s"], 41 / 60)
        self.assertEqual(load["latency_ms"]["p50"], 2000.0)
        self.assertEqual(load["consensus_latency_ms"]["p50"], 1500.0)
        self.assertAlmostEqual(load["query_lag_ms"]["p50"], 500.0)
        self.assertAlmostEqual(load["query_lag_end_ms"], 500.0)
        self.assertAlmostEqual(load["tracker_lag_ms"]["p99"], 100.0)
        # 41 spans of 2 s and one of 30 s over 60 s.
        self.assertAlmostEqual(load["in_flight_mean"], 112 / 60)
        self.assertEqual(load["timeouts"], 1)
        self.assertEqual(load["at_cap_frac"], 0.0)
        self.assertAlmostEqual(net["block_bytes_nonempty_mean"], 500_000.0)
        self.assertEqual(net["max_block_bytes"], 100_000_000)
        self.assertEqual(load["offered_mb_s"], 1.0)
        self.assertEqual(result["bound"]["kind"], "keeping-up")
        self.assertEqual(
            result["validity"],
            {"valid": True, "noisy": True, "reasons": ["1 transactions timed out"]},
        )


class PaceTest(unittest.TestCase):
    def test_interval_from_rate(self):
        cfg = bench.BenchConfig(tx_size=1_000_000, rate_mb_s=20.0)
        self.assertAlmostEqual(bench.tx_interval_s(cfg), 0.05)

    def test_on_time_keeps_the_schedule(self):
        self.assertEqual(bench.next_due(10.0, 0.5, 10.2), 10.5)

    def test_late_restarts_from_now(self):
        self.assertEqual(bench.next_due(10.0, 0.5, 12.0), 12.0)


class BoundTest(unittest.TestCase):
    def kind(self, decided=10.0, at_cap=0.0, empty=0.0):
        return bench.throughput_bound(10.0, decided, at_cap, empty)

    def test_keeping_up(self):
        self.assertEqual(self.kind()["kind"], "keeping-up")
        self.assertEqual(
            self.kind(decided=9.6, at_cap=0.01, empty=0.04)["kind"], "keeping-up"
        )

    def test_decided_below_offered_is_behind(self):
        bound = self.kind(decided=9.0)
        self.assertEqual(bound["kind"], "behind")
        self.assertIn("decided 90% of offered", bound["reason"])

    def test_cap_blocking_is_behind(self):
        bound = self.kind(at_cap=0.1)
        self.assertEqual(bound["kind"], "behind")
        self.assertIn("max_pending", bound["reason"])

    def test_empty_blocks_are_behind(self):
        bound = self.kind(empty=0.5)
        self.assertEqual(bound["kind"], "behind")
        self.assertIn("50% of blocks empty", bound["reason"])

    def test_in_flight_samples_the_window(self):
        spans = [(0.0, 10.0), (0.0, 5.0), (20.0, 30.0)]
        self.assertEqual(
            bench.in_flight(spans, 0.0, 10.0, 1.0), [2, 2, 2, 2, 2, 1, 1, 1, 1, 1]
        )

    def test_parse_size(self):
        self.assertEqual(bench.parse_size("100mb"), 100_000_000)
        self.assertEqual(bench.parse_size("2 KB"), 2000)
        self.assertEqual(bench.parse_size(30720), 30720)
        with self.assertRaises(ValueError):
            bench.parse_size("1 wei")


class StatTest(unittest.TestCase):
    def test_empty_sub_window_gives_json_null(self):
        value = bench.stat([2.0, None], "ms", "lower")
        self.assertIsNone(value["value"])
        self.assertEqual(json.loads(json.dumps(value, allow_nan=False)), value)


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


if __name__ == "__main__":
    unittest.main()
