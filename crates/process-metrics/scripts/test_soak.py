"""Tests for soak.render_summary."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))

from soak import render_summary  # noqa: E402

TMP_ROOT = SCRIPT_DIR.parents[2] / "tmp"


def _mk_docker_row(ts: int, name: str, mem_mib: float, cpu: float) -> dict:
    return {
        "ts": ts,
        "Name": name,
        "MemUsage": f"{mem_mib}MiB / 16GiB",
        "MemPerc": f"{(mem_mib / 16384.0) * 100:.2f}%",
        "CPUPerc": f"{cpu}%",
        "BlockIO": "0B / 0B",
        "NetIO": "0B / 0B",
        "PIDs": "1",
    }


def _write_jsonl(path: Path, rows: list[dict]) -> None:
    path.write_text("\n".join(json.dumps(r) for r in rows) + ("\n" if rows else ""))


class RenderSummaryTests(unittest.TestCase):
    def setUp(self) -> None:
        TMP_ROOT.mkdir(exist_ok=True)
        self._tmp_ctx = tempfile.TemporaryDirectory(dir=TMP_ROOT)
        self.tmp = Path(self._tmp_ctx.name)
        self.docker = self.tmp / "docker-stats.jsonl"
        self.metrics = self.tmp / "node-metrics.jsonl"

    def tearDown(self) -> None:
        self._tmp_ctx.cleanup()

    def test_REQ_soak_summary_format_ok(self) -> None:
        """REQ:soak-summary-format-ok"""
        services = ["espresso-node-0", "espresso-node-1", "espresso-node-2"]
        base_ts = 1_779_259_000
        rows = [
            _mk_docker_row(base_ts + i, name, 100 + j * 50 + i, 10.0 + j)
            for i in range(5)
            for j, name in enumerate(services)
        ]
        _write_jsonl(self.docker, rows)
        metric_rows = [
            {
                "ts": base_ts + i,
                "node": f"http://localhost:{24000 + j}",
                "metric": "process_resident_memory_bytes",
                "value": (100 + j * 50 + i) * 1024 * 1024,
            }
            for i in range(5)
            for j in range(3)
        ]
        _write_jsonl(self.metrics, metric_rows)

        out = render_summary(self.docker, self.metrics, "drb-header", 4)

        self.assertIn("## Memory soak: drb-header", out)
        self.assertIn("**Peak total memory:", out)
        self.assertIn("| Service", out)
        for name in services:
            self.assertIn(name, out)
        self.assertIn("**Total (per-ts sum)**", out)
        self.assertIn("### Node RSS cross-check", out)

    def test_EDGE_soak_summary_sparse_rows(self) -> None:
        """EDGE:soak-summary-empty-rows — one container died early."""
        base_ts = 1_779_259_000
        rows = [
            _mk_docker_row(base_ts + i, "espresso-node-0", 100 + i, 10.0)
            for i in range(5)
        ] + [
            _mk_docker_row(base_ts + i, "espresso-node-1", 200 + i, 20.0)
            for i in range(2)
        ]
        _write_jsonl(self.docker, rows)

        out = render_summary(self.docker, self.metrics, "sparse", 4)
        self.assertIn("espresso-node-0", out)
        self.assertIn("espresso-node-1", out)
        self.assertIn("**Total (per-ts sum)**", out)

    def test_EDGE_soak_summary_no_node_metrics(self) -> None:
        """EDGE:soak-summary-no-node-metrics — metrics file missing."""
        base_ts = 1_779_259_000
        rows = [
            _mk_docker_row(base_ts + i, "espresso-node-0", 100 + i, 10.0)
            for i in range(3)
        ]
        _write_jsonl(self.docker, rows)
        self.assertFalse(self.metrics.exists())

        out = render_summary(self.docker, self.metrics, "no-metrics", 2)
        self.assertIn("| Service", out)
        self.assertNotIn("### Node RSS cross-check", out)


if __name__ == "__main__":
    unittest.main()
