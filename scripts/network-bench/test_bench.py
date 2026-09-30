"""Tests for `bench`: process lifecycle, preflight, CLI wiring. No network is started."""

import dataclasses
import importlib.util
import json
import logging
import subprocess
import types
from importlib.machinery import SourceFileLoader
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import FakeClock, make_result, step

_spec = importlib.util.spec_from_loader(
    "bench", SourceFileLoader("bench", str(Path(__file__).with_name("bench")))
)
assert _spec is not None and _spec.loader is not None
bench = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(bench)


def invalid_result() -> netbench.BenchResult:
    result = make_result()
    result["validity"] = {"valid": False, "noisy": False, "reasons": ["no blocks"]}
    return result


@pytest.mark.parametrize(
    ("current", "code", "summary_has"),
    [
        (make_result([step(4.0, 3.0, ["x"])]), 0, "**worse**"),
        (invalid_result(), 1, None),
    ],
    ids=["regression", "invalid"],
)
def test_write_report_exit_code(
    tmp_path, monkeypatch, capsys, current, code, summary_has
):
    (tmp_path / "config.json").write_text(
        json.dumps(dataclasses.asdict(netbench.BenchConfig()))
    )
    baseline = tmp_path / "baseline.json"
    baseline.write_text(json.dumps({"runs": [make_result()] * 3}))
    monkeypatch.setattr(netbench, "analyze", lambda *_: current)
    assert bench.write_report(tmp_path, netbench.load_baseline(baseline)) == code
    if summary_has:
        assert summary_has in (tmp_path / "summary.md").read_text()


def test_compare_with_empty_baseline_runs_exits_zero(tmp_path, capsys):
    result, baseline = tmp_path / "result.json", tmp_path / "baseline.json"
    result.write_text(json.dumps(make_result()))
    baseline.write_text(json.dumps({"runs": [], "error": "none found"}))
    args = bench.parse_args(["compare", str(result), "--baseline", str(baseline)])
    assert bench.cmd_compare(args) == 0


def test_preflight_reports_busy_ports_and_running_processes(monkeypatch):
    monkeypatch.setattr(bench, "port_in_use", lambda port: port == 7)
    monkeypatch.setattr(bench, "proc_comms", lambda: iter([(9, "anvil"), (10, "vim")]))
    monkeypatch.setattr(bench, "proc_uid", lambda pid: bench.os.getuid())
    assert bench.preflight_problems((6, 7), ("anvil",)) == [
        "port 7 is in use",
        "anvil (pid 9) is still running",
    ]


def test_bad_baseline_fails_before_starting_network(tmp_path, monkeypatch):
    baseline = tmp_path / "baseline.json"
    baseline.write_text('{"runs": [')
    args = bench.parse_args(
        ["run", "--bin-dir", "/nonexistent", "--baseline", str(baseline)]
    )
    monkeypatch.setattr(bench, "start_network", pytest.fail)
    with pytest.raises(json.JSONDecodeError):
        bench.cmd_run(args)


def test_run_refuses_without_starting_network(monkeypatch, caplog):
    args = bench.parse_args(["run", "--bin-dir", "/nonexistent"])
    monkeypatch.setattr(bench, "preflight_problems", lambda *_: ["port 1 is in use"])
    monkeypatch.setattr(bench, "start_network", pytest.fail)
    with caplog.at_level(logging.ERROR, bench.log.name):
        assert bench.cmd_run(args) == 2
    assert "scripts/cleanup-process-compose" in caplog.text


def test_unexpected_error_tears_down_and_writes_failure_summary(
    tmp_path, monkeypatch, capsys
):
    out, storage = tmp_path / "out", tmp_path / "storage"
    storage.mkdir()
    env_file = tmp_path / ".env"
    env_file.write_text("A=1\n")
    net = bench.Network(proc=object(), out=tmp_path, storage=tmp_path)
    torn_down = []

    def wait_ready(*_):
        raise ValueError("bad height")

    calib = {"sha256_1t_mb_s": 1.0, "sha256_mt_mb_s": 1.0, "fsync_per_s": 1.0}
    stubs: dict[Any, dict[str, Any]] = {
        bench: {
            "preflight_problems": lambda *_: [],
            "collect_sysinfo": lambda *_: (make_result()["runner"], {}),
            "make_storage": lambda *_: storage,
            "remove_storage": lambda *_: None,
            "start_network": lambda *_, **__: net,
            "calibrate": lambda *_: calib,
            "sample_host": lambda *_, **__: None,
            "teardown": lambda n: torn_down.append(n) or [],
            "ENV_FILE": env_file,
        },
        netbench: {
            "sample_metrics": lambda *_, **__: None,
            "wait_ready": wait_ready,
        },
    }
    for module, attrs in stubs.items():
        for name, value in attrs.items():
            monkeypatch.setattr(module, name, value)

    args = bench.parse_args(["run", "--bin-dir", "/nonexistent", "--out", str(out)])
    assert bench.cmd_run(args) == 1
    assert torn_down == [net]
    run = json.loads((out / "run.json").read_text())
    assert run["error"] == "ValueError: bad height"
    assert "**invalid**: ValueError: bad height" in (out / "summary.md").read_text()


class StuckProc:
    pid = 77

    def __init__(self) -> None:
        self.killed = False

    def poll(self) -> int | None:
        return 0 if self.killed else None

    def send_signal(self, _sig: int) -> None:
        pass

    def wait(self, timeout: float | None = None) -> int:
        if timeout is not None:
            raise subprocess.TimeoutExpired("process-compose", timeout)
        return 0


def test_hung_stop_is_force_killed_and_listed(tmp_path, monkeypatch, caplog):
    """Every teardown step is bounded and a failing step skips none."""
    (tmp_path / "logs").mkdir()
    proc, clock = StuckProc(), FakeClock()
    kills: list[tuple[str, int]] = []

    def run(*_, **__):
        raise subprocess.TimeoutExpired("cleanup", 1)

    def killpg(pid: int, _sig: int) -> None:
        proc.killed = True
        kills.append(("killpg", pid))

    def kill(pid: int, _sig: int) -> None:
        kills.append(("kill", pid))

    leftovers = iter([[], [(1, "anvil")]])
    monkeypatch.setattr(bench, "STOP_TIMEOUT_S", 0.5)
    monkeypatch.setattr(bench, "own_processes", lambda _: next(leftovers))
    monkeypatch.setattr(bench, "time", clock)
    monkeypatch.setattr(
        bench,
        "subprocess",
        types.SimpleNamespace(
            run=run, TimeoutExpired=subprocess.TimeoutExpired, STDOUT=subprocess.STDOUT
        ),
    )
    monkeypatch.setattr(
        bench, "os", types.SimpleNamespace(killpg=killpg, kill=kill, getuid=lambda: 0)
    )
    net = bench.Network(proc=proc, out=tmp_path, storage=tmp_path)
    with caplog.at_level(logging.WARNING, bench.log.name):
        notes = bench.teardown(net)
    assert kills == [("killpg", 77), ("kill", 1)]
    assert notes == [
        "process-compose did not stop within 0.5 s",
        f"cleanup-process-compose ran over {bench.CLEANUP_TIMEOUT_S} s",
        "force-killed anvil (pid 1)",
    ]


def test_pid_relabelled_after_exec(monkeypatch):
    comms: list[tuple[int, str]] = []
    monkeypatch.setattr(bench, "proc_comms", lambda: iter(comms))
    monkeypatch.setattr(
        bench,
        "process_label",
        lambda pid, comm, *_: bench.PROCESS_LABELS.get(comm),
    )
    cache: dict = {}
    for comm, want in (("bash", []), ("anvil", [(7, "anvil")])):
        comms[:] = [(7, comm)]
        assert list(bench.labelled_processes(cache, Path("/"))) == want
