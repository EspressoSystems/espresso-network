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
        # Docker names mirror docker-compose: <project>-<service>-<idx>.
        services = [
            "espresso-network-espresso-node-0-1",
            "espresso-network-espresso-node-1-1",
            "espresso-network-espresso-node-2-1",
        ]
        base_ts = 1_779_259_000
        rows = [
            _mk_docker_row(base_ts + i, name, 100 + j * 50 + i, 10.0 + j)
            for i in range(5)
            for j, name in enumerate(services)
        ]
        # Add an unrelated container and a phantom empty-Name row.
        rows.extend(
            _mk_docker_row(base_ts + i, "espresso-network-keydb-1", 5 + i, 1.0)
            for i in range(5)
        )
        rows.extend(_mk_docker_row(base_ts + i, "", 0, 0.0) for i in range(5))
        _write_jsonl(self.docker, rows)
        # Process gauge only for nodes 0 and 1.
        metric_rows = [
            {
                "ts": base_ts + i,
                "node": f"espresso-node-{j}",
                "metric": "process_resident_memory_bytes",
                "value": (100 + j * 50 + i) * 1024 * 1024,
            }
            for i in range(5)
            for j in range(2)
        ]
        _write_jsonl(self.metrics, metric_rows)

        out = render_summary(self.docker, self.metrics, "drb-header", 4, self.tmp)

        self.assertIn("## Memory soak: drb-header", out)
        self.assertIn("| Service", out)
        self.assertIn("Max RSS (docker)", out)
        self.assertIn("Max RSS (process gauge)", out)
        # Espresso-node rows present with short names.
        for name in ("espresso-node-0", "espresso-node-1", "espresso-node-2"):
            self.assertIn(name, out)
        # Non-node containers and empty-name phantoms are filtered out.
        self.assertNotIn("keydb", out)
        # Node 2 has no process gauge data.
        self.assertRegex(out, r"\|\s*espresso-node-2\s*\|[^|]*\|\s*n/a\s*\|")
        self.assertIn("**Total (sum)**", out)
        # Mermaid chart has a deterministic palette and a flowchart legend
        # whose node fills come from the same palette.
        self.assertIn("plotColorPalette", out)
        self.assertIn("flowchart LR", out)
        for i, name in enumerate(
            ("espresso-node-0", "espresso-node-1", "espresso-node-2")
        ):
            self.assertRegex(out, rf'n{i}\["{name}"\]')
            self.assertRegex(out, rf"style n{i} fill:#[0-9a-fA-F]+")
        # No legacy headings/columns.
        self.assertNotIn("Node RSS cross-check", out)
        self.assertNotIn("p99 RSS", out)
        self.assertNotIn("Total (per-ts sum)", out)

    def test_EDGE_soak_summary_sparse_rows(self) -> None:
        """EDGE:soak-summary-empty-rows — one container died early."""
        base_ts = 1_779_259_000
        rows = [
            _mk_docker_row(
                base_ts + i, "espresso-network-espresso-node-0-1", 100 + i, 10.0
            )
            for i in range(5)
        ] + [
            _mk_docker_row(
                base_ts + i, "espresso-network-espresso-node-1-1", 200 + i, 20.0
            )
            for i in range(2)
        ]
        _write_jsonl(self.docker, rows)

        out = render_summary(self.docker, self.metrics, "sparse", 4, self.tmp)
        self.assertIn("espresso-node-0", out)
        self.assertIn("espresso-node-1", out)
        self.assertIn("**Total (sum)**", out)

    def test_EDGE_soak_summary_gib_units(self) -> None:
        """EDGE:soak-summary-gib-units - docker emits GiB for large containers."""
        base_ts = 1_779_259_000
        rows = []
        for i in range(3):
            row = _mk_docker_row(
                base_ts + i, "espresso-network-espresso-node-0-1", 0, 10.0
            )
            row["MemUsage"] = f"{1.5 + 0.01 * i}GiB / 16GiB"
            rows.append(row)
        _write_jsonl(self.docker, rows)

        out = render_summary(self.docker, self.metrics, "gib", 2, self.tmp)
        self.assertIn("espresso-node-0", out)
        self.assertNotIn("nan", out.lower())
        self.assertNotIn("| n/a | n/a |", out)

    def test_EDGE_soak_summary_empty_docker_file(self) -> None:
        """EDGE:soak-summary-empty-docker - empty JSONL must not crash."""
        self.docker.write_text("")
        out = render_summary(self.docker, self.metrics, "empty", 0, self.tmp)
        self.assertIn("No data collected", out)

    def test_EDGE_soak_summary_no_node_metrics(self) -> None:
        """EDGE:soak-summary-no-node-metrics - metrics file missing."""
        base_ts = 1_779_259_000
        rows = [
            _mk_docker_row(
                base_ts + i, "espresso-network-espresso-node-0-1", 100 + i, 10.0
            )
            for i in range(3)
        ]
        _write_jsonl(self.docker, rows)
        self.assertFalse(self.metrics.exists())

        out = render_summary(self.docker, self.metrics, "no-metrics", 2, self.tmp)
        self.assertIn("| Service", out)
        self.assertNotIn("Node RSS cross-check", out)
        # Process gauge column shows n/a with no metrics file.
        self.assertRegex(out, r"\|\s*espresso-node-0\s*\|[^|]*\|\s*n/a\s*\|")


if __name__ == "__main__":
    unittest.main()
