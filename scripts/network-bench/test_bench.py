"""Tests for `bench`: process lifecycle, preflight, CLI wiring. No network is started.

just py::test
"""

import contextlib
import dataclasses
import importlib.util
import json
import socket
import subprocess
import sys
import tempfile
import unittest
from importlib.machinery import SourceFileLoader
from pathlib import Path
from unittest import mock

import netbench
from test_netbench import make_result, step

SCRIPT = Path(__file__).with_name("bench")
_spec = importlib.util.spec_from_loader("bench", SourceFileLoader("bench", str(SCRIPT)))
assert _spec is not None
bench = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(bench)


class ReportOnlyTest(unittest.TestCase):
    def report(self, current):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "config.json").write_text(
                json.dumps(dataclasses.asdict(netbench.BenchConfig()))
            )
            baseline = out / "baseline.json"
            baseline.write_text(json.dumps({"runs": [make_result()] * 3}))
            with (
                mock.patch.object(netbench, "analyze", return_value=current),
                mock.patch("builtins.print"),
                self.assertLogs(bench.log),
            ):
                code = bench.write_report(out, netbench.load_baseline(baseline))
            return code, (out / "summary.md").read_text()

    def test_regression_still_exits_zero(self):
        code, summary = self.report(make_result([step(4.0, 3.0, ["x"])]))
        self.assertEqual(code, 0)
        self.assertIn("**worse**", summary)

    def test_invalid_run_exits_one(self):
        current = make_result()
        current["validity"] = {"valid": False, "noisy": False, "reasons": ["no blocks"]}
        self.assertEqual(self.report(current)[0], 1)


class CmdCompareTest(unittest.TestCase):
    def test_compare_cli_with_empty_runs_exits_zero(self):
        with tempfile.TemporaryDirectory() as tmp:
            result, baseline = Path(tmp) / "result.json", Path(tmp) / "baseline.json"
            result.write_text(json.dumps(make_result()))
            baseline.write_text(json.dumps({"runs": [], "error": "none found"}))
            args = bench.parse_args(
                ["compare", str(result), "--baseline", str(baseline)]
            )
            with mock.patch("builtins.print"):
                self.assertEqual(bench.cmd_compare(args), 0)


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
                # Moved symbols are called through `nb.`; patch them on netbench.
                target = bench if hasattr(bench, name) else netbench
                stack.enter_context(mock.patch.object(target, name, stub))
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


if __name__ == "__main__":
    unittest.main()
