"""Tests for `bench`: rendering, comparison, validity, and the load generator against a fake node.

No network is started.

    just py::test
"""

import argparse
import asyncio
import base64
import importlib.util
import json
import socket
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
        "config": {"workers": 6, "max_pending": 48, "tx_size": 100_000},
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
            "latency_ms": quantiles(1200.0, 2500.0),
        },
        "stake_table": ["0x1", "0x1", "0x1"],
        "validity": {"valid": True, "noisy": False, "reasons": []},
    }
    result["validity"] = bench.check_validity(result, {n: 1.0 for n in bench.NODES})
    return result


def row(comparison, label):
    return next(r for r in comparison["rows"] if r["label"] == label)


class RenderTest(unittest.TestCase):
    def test_summary_has_headline_and_delta(self):
        baseline = [make_result(mb_per_s=4.0) for _ in range(5)]
        current = make_result(mb_per_s=3.0)
        summary = bench.render(current, bench.compare(current, baseline))
        self.assertIn(
            "| metric | this run | sub-window range | main median (n=5) | delta |",
            summary,
        )
        self.assertRegex(
            summary, r"\| decided throughput \| 3 MB/s \|.*\| -25\.0% \| \*\*worse\*\*"
        )
        self.assertIn("Test CPU", summary)

    def test_no_baseline(self):
        current = make_result()
        comparison = bench.compare(current, [])
        self.assertEqual(comparison["n"], 0)
        self.assertIn(
            "No baseline: 0 comparable main runs", bench.render(current, comparison)
        )
        self.assertIn("No baseline given.", bench.render(current, None))

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
        comparison = bench.compare(current, [make_result(mb_per_s=4.0)])
        self.assertEqual(row(comparison, "decided throughput")["verdict"], "same")

    def test_spread_widens_threshold(self):
        history = [make_result(mb_per_s=v) for v in (3.0, 4.0, 5.0, 3.5, 4.5)]
        verdict = row(
            bench.compare(make_result(mb_per_s=3.0), history), "decided throughput"
        )
        self.assertGreater(verdict["threshold_pct"], 10)
        self.assertEqual(verdict["verdict"], "same")

    def test_excludes_noisy_invalid_and_other_config(self):
        noisy = make_result(steal=8.0)
        invalid = make_result()
        invalid["validity"] = {"valid": False, "noisy": False, "reasons": ["x"]}
        other = make_result(config_hash="other")
        comparison = bench.compare(
            make_result(), [noisy, invalid, other, make_result()]
        )
        self.assertEqual((comparison["n"], comparison["excluded"]), (1, 3))

    def test_noisy_current_is_inconclusive(self):
        current = make_result(mb_per_s=1.0, steal=8.0)
        comparison = bench.compare(current, [make_result()])
        self.assertEqual(
            row(comparison, "decided throughput")["verdict"], "inconclusive"
        )

    def test_timeouts_from_zero_baseline(self):
        current = make_result()
        current["network"]["timeouts"] = 3
        self.assertEqual(
            row(bench.compare(current, [make_result()]), "timeouts")["verdict"], "worse"
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


class ReportOnlyTest(unittest.TestCase):
    def report(self, current):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "config.json").write_text(json.dumps({"workers": 6}))
            baseline = out / "baseline.json"
            baseline.write_text(json.dumps({"runs": [make_result(mb_per_s=8.0)] * 3}))
            with (
                mock.patch.object(bench, "analyze", return_value=current),
                mock.patch("builtins.print"),
                self.assertLogs(bench.log),
            ):
                code = bench.write_report(out, baseline)
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


class FakeNode(ThreadingHTTPServer):
    """Submit, block height and payload endpoints; `include` decides whether blocks carry the
    submitted transactions."""

    def __init__(self, include):
        super().__init__(("127.0.0.1", 0), FakeHandler)
        self.include = include
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
            if height >= len(node.blocks):
                return self.reply(404, "not found")
            raw = base64.b64encode(node.blocks[height]).decode()
        self.reply(200, {"data": {"raw_payload": raw, "ns_table": {"bytes": ""}}})


class LoadTest(unittest.TestCase):
    def run_load(self, include, duration, **cfg):
        node = FakeNode(include)
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
        self.assertGreater(len(txs), 4 * 3)
        self.assertTrue(all(tx["status"] == "included" for tx in txs))
        self.assertLessEqual(meta["max_in_flight"], 4)
        self.assertLessEqual(node.max_outstanding, 4)

    def test_timeout_releases_permits(self):
        start, node, txs, meta = self.run_load(
            False, 2.5, max_pending=4, tx_timeout_s=1
        )
        before_first_timeout = [t for t in node.submits if t < start + 1.0]
        self.assertEqual(len(before_first_timeout), 4)
        self.assertGreater(len(node.submits), 4)
        self.assertEqual(meta["max_in_flight"], 4)
        self.assertTrue(all(tx["status"] == "timeout" for tx in txs))


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
