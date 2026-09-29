"""Tests for `bench`: rendering, comparison, validity, and the load generator against a fake node.

No network is started.

    just py::test
"""

import argparse
import asyncio
import base64
import dataclasses
import importlib.util
import json
import socket
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
            "missing_payloads": [],
            "latency_ms": quantiles(1200.0, 2500.0),
        },
        "bound": {"kind": "unclear", "reason": "r"},
        "stake_table": ["0x1", "0x1", "0x1"],
        "validity": {"valid": True, "noisy": False, "reasons": []},
    }
    result["validity"] = bench.check_validity(result, {n: 1.0 for n in bench.NODES})
    return result


def compare(current, runs, error=None):
    return bench.compare(current, {"runs": runs, "error": error})


def row(comparison, label):
    return next(r for r in comparison["rows"] if r["label"] == label)


class RenderTest(unittest.TestCase):
    def test_summary_has_headline_and_delta(self):
        baseline = [make_result(mb_per_s=4.0) for _ in range(5)]
        current = make_result(mb_per_s=3.0)
        summary = bench.render(current, compare(current, baseline))
        self.assertIn(
            "| metric | this run | sub-window range | baseline median (n=5) | delta |",
            summary,
        )
        self.assertRegex(
            summary, r"\| decided throughput \| 3 MB/s \|.*\| -25\.0% \| \*\*worse\*\*"
        )
        self.assertIn("Test CPU", summary)

    def test_status_names_the_pr(self):
        current = make_result()
        current["run"] |= {"pr": 42, "event": "workflow_dispatch"}
        self.assertIn("`0123456789` PR #42, config", bench.render(current, None))

    def test_no_baseline(self):
        current = make_result()
        comparison = compare(current, [])
        self.assertEqual(comparison["n"], 0)
        self.assertIn(
            "No baseline: 0 comparable runs", bench.render(current, comparison)
        )
        self.assertIn("No baseline given.", bench.render(current, None))

    def test_failed_fetch_is_not_no_baseline(self):
        current = make_result()
        summary = bench.render(current, compare(current, [], "OSError: timed out"))
        self.assertIn("Baseline fetch failed: OSError: timed out", summary)
        self.assertNotIn("No baseline", summary)

    def test_load_baseline_single_result(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "result.json"
            path.write_text(json.dumps(make_result()))
            self.assertEqual(len(bench.load_baseline(path)["runs"]), 1)
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


class CompareTest(unittest.TestCase):
    def test_within_threshold_is_same(self):
        current = make_result(mb_per_s=3.8)
        comparison = compare(current, [make_result(mb_per_s=4.0)])
        self.assertEqual(row(comparison, "decided throughput")["verdict"], "same")

    def test_spread_widens_threshold(self):
        history = [make_result(mb_per_s=v) for v in (3.0, 4.0, 5.0, 3.5, 4.5)]
        verdict = row(compare(make_result(mb_per_s=3.0), history), "decided throughput")
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
            "No baseline: 0 comparable runs (excluded: 1 other runner)",
            bench.render(make_result(), compare(make_result(), [other_cpu])),
        )

    def test_noisy_current_is_inconclusive(self):
        current = make_result(mb_per_s=1.0, steal=8.0)
        comparison = compare(current, [make_result()])
        self.assertEqual(
            row(comparison, "decided throughput")["verdict"], "inconclusive"
        )

    def test_timeouts_from_zero_baseline(self):
        current = make_result()
        current["network"]["timeouts"] = 3
        self.assertEqual(
            row(compare(current, [make_result()]), "timeouts")["verdict"], "worse"
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
    submitted transactions."""

    def __init__(self, include, lost=frozenset()):
        super().__init__(("127.0.0.1", 0), FakeHandler)
        self.include = include
        self.lost = lost
        self.lock = threading.Lock()
        self.pending = []
        self.blocks = [b""]
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


class FakeHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
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
        with node.lock:
            node.pending.append(base64.b64decode(tx["payload"]))
            node.submits.append(time.time())
            node.max_outstanding = max(node.max_outstanding, len(node.pending))
        self.reply(200, "TX~fake")

    def do_GET(self):
        node = self.server
        with node.lock:
            if self.path == "/v1/node/block-height":
                return self.reply(200, len(node.blocks))
            height = int(self.path.removeprefix("/v1/availability/payload/"))
            if height >= len(node.blocks) or height in node.lost:
                return self.reply(404, "not found")
            raw = base64.b64encode(node.blocks[height]).decode()
        self.reply(200, {"data": {"raw_payload": raw, "ns_table": {"bytes": ""}}})


class LoadTest(unittest.TestCase):
    def run_load(self, include, duration, lost=frozenset(), **cfg):
        node = FakeNode(include, lost)
        threads = [
            threading.Thread(target=node.serve_forever),
            threading.Thread(target=node.produce),
        ]
        for thread in threads:
            thread.start()
        config = bench.BenchConfig(tx_size=1000, workers=3, **cfg)
        try:
            with tempfile.TemporaryDirectory() as tmp:
                start = time.time()
                asyncio.run(
                    bench.generate_load(
                        config, [node.url], node.url, start + duration, Path(tmp)
                    )
                )
                lines = (Path(tmp) / "load.jsonl").read_text().splitlines()
                txs = [json.loads(line) for line in lines]
                meta = json.loads((Path(tmp) / "load-meta.json").read_text())
        finally:
            node.stop.set()
            node.shutdown()
            for thread in threads:
                thread.join()
            node.server_close()
        return start, node, txs, meta

    def test_inclusion_releases_permits(self):
        _, node, txs, meta = self.run_load(True, 1.0, max_pending=4, tx_timeout_s=5)
        self.assertGreater(len(txs), 4)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))
        self.assertLessEqual(meta["max_in_flight"], 4)
        self.assertLessEqual(node.max_outstanding, 4)

    def test_timeout_releases_permits(self):
        _, node, txs, meta = self.run_load(False, 2.5, max_pending=4, tx_timeout_s=1)
        # A permit returns only on timeout, and the first one is 1 s after the first submit.
        self.assertGreater(node.submits[4] - node.submits[0], 0.9)
        self.assertGreater(len(node.submits), 4)
        self.assertEqual(meta["max_in_flight"], 4)
        self.assertTrue(all(tx["status"] == "timeout" for tx in txs))

    def test_lost_payload_is_skipped(self):
        with (
            mock.patch.object(bench, "MISSING_PAYLOAD_S", 0.2),
            self.assertLogs(bench.log, "WARNING"),
        ):
            _, _, txs, meta = self.run_load(
                True, 1.0, lost={3}, max_pending=4, tx_timeout_s=1
            )
        self.assertEqual(meta["missing_payloads"], [3])
        self.assertGreater(sum(tx["status"] == "included" for tx in txs), 4)


def write_run_dir(out):
    """A 60 s window (t 100 to 160) where every node decides 1 MB/s in 2 blocks/s and 4 views/s
    and uses 0.5 cores, the host is half busy, and 41 of 42 transactions land 2 s after submit."""
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
        {"id": i, "node": 0, "t_submit": t, "t_included": t + 2.0, "status": "included"}
        for i, t in enumerate(range(110, 151))
    ]
    txs.append(
        {
            "id": 99,
            "node": 0,
            "t_submit": 120.5,
            "t_included": None,
            "status": "timeout",
        }
    )
    jsonl("load.jsonl", txs)
    calib = {"sha256_1t_mb_s": 2000.0, "sha256_mt_mb_s": 8000.0, "fsync_per_s": 300.0}
    files = {
        "load-meta.json": {
            "submit_errors": 0,
            "max_in_flight": 4,
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
        cfg = bench.BenchConfig(measure_s=60, subwindow_s=20)
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
        self.assertEqual(load["timeouts"], 1)
        self.assertEqual(load["at_cap_frac"], 0.0)
        self.assertAlmostEqual(net["block_bytes_nonempty_mean"], 500_000.0)
        self.assertEqual(net["max_block_bytes"], 100_000_000)
        self.assertEqual(result["bound"]["kind"], "unclear")
        self.assertEqual(
            result["validity"],
            {"valid": True, "noisy": True, "reasons": ["1 transactions timed out"]},
        )


class BoundTest(unittest.TestCase):
    def kind(self, at_cap, empty, fill):
        return bench.throughput_bound(at_cap, empty, fill)["kind"]

    def test_empty_blocks_are_load_bound(self):
        self.assertEqual(self.kind(0.78, 0.75, 0.05), "load")
        self.assertEqual(self.kind(0.1, 0.5, 0.05), "load")

    def test_full_blocks_are_network_bound(self):
        self.assertEqual(self.kind(0.9, 0.5, 0.85), "network")

    def test_busy_blocks_at_cap_are_network_bound(self):
        self.assertEqual(self.kind(0.9, 0.01, 0.3), "network")

    def test_between_rules_is_unclear(self):
        self.assertEqual(self.kind(0.9, 0.1, 0.3), "unclear")
        self.assertEqual(self.kind(0.2, 0.01, 0.3), "unclear")

    def test_at_cap_frac_samples_the_window(self):
        spans = [(0.0, 10.0), (0.0, 5.0), (20.0, 30.0)]
        self.assertEqual(bench.at_cap_frac(spans, 0.0, 10.0, 2, 1.0), 0.5)

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
