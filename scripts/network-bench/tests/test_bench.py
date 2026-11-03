import dataclasses
import json
import logging
import subprocess
import types
from pathlib import Path

import netbench
import pytest
from fakes import FakeClock, load_script, make_result, step

bench = load_script("bench")


def report(tmp_path: Path, monkeypatch, current: netbench.BenchResult) -> int:
    netbench.write_json(
        tmp_path / "config.json", dataclasses.asdict(netbench.BenchConfig())
    )
    baseline = tmp_path / "baseline.json"
    netbench.write_json(baseline, {"runs": [make_result()] * 3})
    monkeypatch.setattr(netbench, "analyze", lambda *_: current)
    return bench.write_report(tmp_path, netbench.load_baseline(baseline))


def test_a_regression_exits_zero_and_is_marked_worse(tmp_path, monkeypatch):
    assert report(tmp_path, monkeypatch, make_result([step(4.0, 3.0, ["x"])])) == 0
    assert "**worse**" in (tmp_path / "summary.md").read_text()


def test_an_invalid_result_exits_one(tmp_path, monkeypatch):
    result = make_result()
    result["validity"] = {"valid": False, "noisy": False, "reasons": ["no blocks"]}
    assert report(tmp_path, monkeypatch, result) == 1


def test_compare_with_empty_baseline_runs_exits_zero(tmp_path):
    result, baseline = tmp_path / "result.json", tmp_path / "baseline.json"
    netbench.write_json(result, make_result())
    netbench.write_json(baseline, {"runs": [], "error": "none found"})
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


@pytest.mark.parametrize(
    ("flags", "keep_going"), [((), False), (("--keep-going",), True)]
)
def test_keep_going_flag(flags, keep_going):
    args = bench.parse_args(["run", "--bin-dir", "/nonexistent", *flags])
    assert bench.config_from_args(args).keep_going == keep_going


def test_run_refuses_without_starting_network(monkeypatch, caplog):
    args = bench.parse_args(["run", "--bin-dir", "/nonexistent"])
    monkeypatch.setattr(bench, "preflight_problems", lambda *_: ["port 1 is in use"])
    monkeypatch.setattr(bench, "start_network", pytest.fail)
    with caplog.at_level(logging.ERROR, bench.log.name):
        assert bench.cmd_run(args) == 2
    assert "scripts/cleanup-process-compose" in caplog.text


@pytest.fixture
def stubbed_run(tmp_path, monkeypatch) -> list:
    """`cmd_run` without a network: `wait_ready` fails. Returns the torn down networks."""
    storage = tmp_path / "storage"
    storage.mkdir()
    env_file = tmp_path / ".env"
    env_file.write_text("A=1\n")
    net = bench.Network(proc=object(), out=tmp_path, storage=tmp_path)
    torn_down: list = []
    calib = {"sha256_1t_mb_s": 1.0, "sha256_mt_mb_s": 1.0, "fsync_per_s": 1.0}

    def wait_ready(*_):
        raise ValueError("bad height")

    monkeypatch.setattr(bench, "preflight_problems", lambda *_: [])
    monkeypatch.setattr(
        bench, "collect_sysinfo", lambda *_: (make_result()["runner"], {})
    )
    monkeypatch.setattr(bench, "make_storage", lambda *_: storage)
    monkeypatch.setattr(bench, "remove_storage", lambda *_: None)
    monkeypatch.setattr(bench, "start_network", lambda *_, **__: net)
    monkeypatch.setattr(bench, "calibrate", lambda *_: calib)
    monkeypatch.setattr(bench, "sample_host", lambda *_, **__: None)
    monkeypatch.setattr(bench, "teardown", lambda n: torn_down.append(n) or [])
    monkeypatch.setattr(bench, "ENV_FILE", env_file)
    monkeypatch.setattr(netbench, "sample_metrics", lambda *_, **__: None)
    monkeypatch.setattr(netbench, "wait_ready", wait_ready)
    return torn_down


def test_unexpected_error_tears_down_and_writes_failure_summary(tmp_path, stubbed_run):
    out = tmp_path / "out"
    args = bench.parse_args(["run", "--bin-dir", "/nonexistent", "--out", str(out)])
    assert bench.cmd_run(args) == 1
    assert len(stubbed_run) == 1
    assert netbench.read_json(out / "run.json")["error"] == "ValueError: bad height"
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
    """TEST:bench-teardown-hang-ok: every teardown step is bounded and a failing step skips
    none."""
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
