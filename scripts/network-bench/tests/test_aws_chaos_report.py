import gzip
import json
from collections import Counter
from datetime import UTC, datetime
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import chaos as ch
import netbench
import pytest
from fakes import (
    FakeRunner,
    FakeSystem,
    awsb,
    index_manifest,
    make_result,
    raiser,
    step,
)
from test_chaos import EVENTS, KINDS, T0, ev

CONFIG = {
    "nodes": 22,
    "query_nodes": 4,
    "node_type": "c8g.xlarge",
    "latency": "decaf-2025",
    "intra_latency": True,
    "chaos": {"minutes": 3, "rate_mb_s": 4.0, "seed": 42, "kinds": KINDS},
}


def spans_of(steps: list[Any]) -> list[tuple[float, float]]:
    return [(s["t_start"], s["t_end"]) for s in steps]


def steps_of(decided: list[float | None]) -> list[Any]:
    steps = [step(4.0, d) for d in decided]
    for i, s in enumerate(steps):
        s["decided_mb_s"] = decided[i]
        s["t_start"], s["t_mid"], s["t_end"] = (
            T0 + 60 * i,
            T0 + 60 * i + 30,
            T0 + 60 * i + 60,
        )
    return steps


def test_throughput_has_the_mean_and_the_lowest_step():
    steps = steps_of([4.0, 2.0, 3.0, 4.0])
    assert awsb.throughput_lines(steps) == [
        (
            "- Throughput: mean decided 3.25 MB/s of 4 MB/s offered over 4 steps; "
            "minimum one-minute 2 MB/s (step at 60 s)"
        )
    ]
    assert awsb.throughput_lines(steps_of([None, None])) == []


def write_counters(run_dir: Path) -> None:
    """Every 10 s: 2 MB/s while a fault of `EVENTS` is active (10-40 s, 70-150 s), else 4."""
    decided, lines = 0.0, []
    for at in range(0, 250, 10):
        lines.append(json.dumps({"ts": T0 + at, "decided_bytes": decided}) + "\n")
        faulty = 10 <= at < 40 or 70 <= at < 150
        decided += 20e6 if faulty else 40e6
    (run_dir / "consensus.jsonl").write_text("".join(lines))


def test_decided_with_and_without_a_fault_comes_from_the_counters(tmp_path: Path):
    steps = steps_of([4.0, 2.0, 3.0, 4.0])
    rows = ch.fault_rows(EVENTS)
    assert awsb.fault_throughput_lines(tmp_path, rows, steps, T0 + 240) == []
    (tmp_path / "consensus.jsonl").write_text("")
    assert awsb.fault_throughput_lines(tmp_path, rows, steps, T0 + 240) == []
    write_counters(tmp_path)
    assert awsb.fault_throughput_lines(tmp_path, rows, steps, T0 + 240) == [
        "- Decided with an active fault 2 MB/s over 110 s, without 4 MB/s over 130 s"
    ]
    assert awsb.fault_throughput_lines(tmp_path, [], steps, T0 + 240) == [
        "- Decided with an active fault - over 0 s, without 3.08 MB/s over 240 s"
    ]


def test_totals_count_faults_budget_timeouts_and_txs():
    meta = {"missing_payloads": [641, 1553], "submit_failovers": 19}
    txs = Counter(included=8, timeout=2, pending=1)
    rows = ch.fault_rows(EVENTS)
    lines = awsb.chaos_totals(rows, EVENTS, T0 + 240, CONFIG, meta, txs)
    assert lines == [
        "- Faults: 2 (restart 1, kill 1, wipe 0); max concurrent faulty 1 of budget 6",
        "- Timeouts: 0; lost payloads 2 (641, 1553); submit failovers 19",
        "- Txs: 11 submitted, 8 included, 2 timed out, 1 pending",
    ]
    assert (
        "lost payloads"
        not in awsb.chaos_totals(rows, EVENTS, T0 + 240, CONFIG, None, txs)[1]
    )


def chaos_result(steps: list[Any]) -> Any:
    result = make_result(steps)
    result["nodes"] = {
        "node0": result["nodes"]["node0"],
        "node5": result["nodes"]["node1"],
    }
    for s in steps:
        s["node_cpu"] = {"node0": -1.0, "node5": 0.5}
    return result


def view_of(steps: list[Any]) -> Any:
    chaos = ch.step_chaos(spans_of(steps), ch.fault_rows(EVENTS), T0, T0 + 240)
    return {
        "lead": ["## Chaos test", "", "- Verdict: **pass**"],
        "faults": [],
        "steps": chaos,
    }


def test_render_leads_with_the_chaos_test_and_lists_every_step():
    steps = steps_of([4.0, 2.0, 3.0, 4.0])
    result = chaos_result(steps)
    text = netbench.render(result, None, "![chart](throughput.png)", view_of(steps))
    assert text.startswith("## Chaos test\n")
    for gone in ("Network benchmark", "Capacity", "Baseline", "search:"):
        assert gone not in text
    assert text.index("![chart]") < text.index("### Load steps")
    table = text.split("### Load steps")[1].split("<details>")[0]
    rows = [ln for ln in table.splitlines() if ln[:3] in ("| 0", "| 6", "| 1")]
    assert [r.split(" | ")[0] for r in rows] == ["| 0", "| 60", "| 120", "| 180"]
    assert [r.rsplit(" | ", 1)[1] for r in rows] == [
        "node0 restart |",
        "node5 kill |",
        "node5 kill |",
        "- |",
    ]


def test_chaos_step_table_shows_view_timeouts_not_a_verdict():
    steps = steps_of([4.0, 2.0, 3.0, 4.0])
    steps[1]["timeouts"], steps[2]["timeouts"] = 3, None
    steps[1]["consensus_fails"] = ["3 view timeouts"]
    table = netbench.chaos_step_table(steps, view_of(steps)["steps"])
    assert "| view timeouts | faults |" in table[0] and "verdict" not in table[0]
    assert [r.rsplit(" | ", 2)[1] for r in table[2:]] == ["0", "3", "-", "0"]
    assert not any("fail" in r or "pass" in r for r in table)


def test_step_details_show_a_dash_for_a_restarted_node_only_under_chaos():
    steps = steps_of([4.0, 4.0, 4.0, 4.0])
    result = chaos_result(steps)
    chaos = view_of(steps)
    detail = netbench.step_details(result, chaos["steps"])
    cpu = [row.split("|")[-4].strip() for row in detail[2:6]]
    assert cpu == ["-/0.5", "-1/-", "-1/-", "-1/0.5"]
    assert all("-1/0.5" in row for row in netbench.step_details(result)[2:6])


def test_step_table_without_chaos_still_collapses_equal_rates():
    result = chaos_result(steps_of([4.0, 4.0]))
    assert len(netbench.step_table(result, None)) == 3


def chaos_manifest() -> Any:
    manifest = index_manifest()
    manifest["config"] = {**manifest["config"], **CONFIG}
    manifest["hosts"] = [{"name": f"node{i}"} for i in (0, 2, 5)]
    return manifest


def write_failed_run(run_dir: Path, png: bool = False) -> None:
    """The files of a run that ended in a gate timeout: no `run.json` or `result.json`, and an
    empty `consensus.jsonl`."""
    run_dir.mkdir(parents=True, exist_ok=True)
    events = [
        *EVENTS,
        ev(130, "fault", "node2", "wipe"),
        ev(430, "timeout", "node2", "wipe"),
    ]
    (run_dir / "chaos.jsonl").write_text("".join(json.dumps(e) + "\n" for e in events))
    netbench.write_json(run_dir / "steps.json", steps_of([None, None]))
    meta = {"missing_payloads": [], "submit_failovers": 2}
    netbench.write_json(run_dir / "load-meta.json", meta)
    (run_dir / "load.jsonl").write_text(
        '{"status": "included"}\n{"status": "pending"}\n'
    )
    netbench.write_json(run_dir / "manifest.json", chaos_manifest())
    for node in ("node0", "node2"):
        log_dir = run_dir / "hosts" / node
        log_dir.mkdir(parents=True)
        (log_dir / "espresso-node.log.gz").write_bytes(
            gzip.compress(f"{node} log\n".encode())
        )
    if png:
        (run_dir / "throughput.png").write_bytes(b"png")


def test_failure_summary_of_a_chaos_run_reports_what_was_collected(tmp_path: Path):
    write_failed_run(tmp_path, png=True)
    manifest = netbench.read_json(tmp_path / "manifest.json")
    awsb.write_failure_summary(tmp_path, manifest, "gate failed", fallback=True)
    text = (tmp_path / "summary.md").read_text()
    assert text.startswith("## Chaos test\n")
    assert "- Verdict: **fail**: node2 (wipe) not recovered after 5 s" in text
    assert "- Faults: 3 (restart 1, kill 1, wipe 1)" in text
    assert "- Txs: 2 submitted, 1 included, 0 timed out, 1 pending" in text
    assert "![throughput" in text and "### Faults" in text
    table = text.split("### Load steps")[1]
    assert table.count("\n| 0 | 4 |") == 1 and "| 60 | 4 |" in table
    assert "Last log lines of node2:" in text and "node2 log" in text
    assert "node0 log" not in text


def log_line(at: float, level: str, message: str, **fields: str) -> str:
    stamp = datetime.fromtimestamp(T0 + at, UTC).isoformat().replace("+00:00", "Z")
    entry = {
        "timestamp": stamp,
        "level": level,
        "fields": {"message": message, **fields},
        "target": "espresso",
    }
    return json.dumps(entry) + "\n"


def test_failure_log_of_a_timed_out_node_has_its_warnings_before_the_timeout(
    tmp_path: Path,
):
    write_failed_run(tmp_path)
    lines = [
        "Starting Espresso node with sqlite...\n",
        log_line(200, "INFO", "info before"),
        log_line(210, "WARN", "fetch failed", err="timeout"),
        log_line(220, "ERROR", "catchup stalled"),
        log_line(431, "WARN", "after the timeout"),
        log_line(500, "INFO", "shutting down"),
    ]
    log = tmp_path / "hosts" / "node2" / "espresso-node.log.gz"
    log.write_bytes(gzip.compress("".join(lines).encode()))
    manifest = netbench.read_json(tmp_path / "manifest.json")
    awsb.write_failure_summary(tmp_path, manifest, "gate failed", fallback=True)
    text = (tmp_path / "summary.md").read_text()
    tail = text.split("WARN and ERROR lines of node2 before its timeout:")[1]
    assert "WARN espresso: fetch failed err=timeout" in tail
    assert "ERROR espresso: catchup stalled" in tail
    for gone in ("info before", "after the timeout", "shutting down", "Last log lines"):
        assert gone not in text


def test_warn_lines_keep_the_last_ones():
    lines = [log_line(i, "WARN", f"w{i}") for i in range(50)]
    picked = awsb.warn_lines(lines, T0 + 40, 30)
    assert len(picked) == 30
    assert picked[-1].endswith("w39") and picked[0].endswith("w10")


def test_failure_summary_without_a_timeout_shows_every_log(tmp_path: Path):
    write_failed_run(tmp_path)
    (tmp_path / "chaos.jsonl").write_text("".join(json.dumps(e) + "\n" for e in EVENTS))
    manifest = netbench.read_json(tmp_path / "manifest.json")
    awsb.write_failure_summary(tmp_path, manifest, "boom", fallback=True)
    text = (tmp_path / "summary.md").read_text()
    assert "node0 log" in text and "node2 log" in text
    assert "- Verdict: **fail**: boom" in text


def test_failure_summary_without_steps_or_load_meta(tmp_path: Path):
    write_failed_run(tmp_path)
    (tmp_path / "steps.json").unlink()
    (tmp_path / "load-meta.json").unlink()
    manifest = netbench.read_json(tmp_path / "manifest.json")
    awsb.write_failure_summary(tmp_path, manifest, "boom", fallback=True)
    text = (tmp_path / "summary.md").read_text()
    assert "### Load steps" not in text and "lost payloads" not in text


def test_failure_summary_of_a_plain_run_is_unchanged(tmp_path: Path):
    manifest = {"hosts": [], "config": {}}
    awsb.write_failure_summary(tmp_path, manifest, "boom", fallback=True)
    assert (tmp_path / "summary.md").read_text() == (
        "## Network benchmark\n\n**invalid**: boom\n"
    )


def test_render_of_a_failed_chaos_run_plots_and_writes_the_report(tmp_path: Path):
    write_failed_run(tmp_path)
    runner = FakeRunner()
    args = awsb.parse_args(["render", str(tmp_path)])
    assert awsb.cmd_render(args, FakeSystem(run=runner)) == awsb.EXIT_FAILED
    assert runner.ran("throughput-plot")
    assert (tmp_path / "summary.md").read_text().startswith("## Chaos test")


def test_report_of_a_failed_chaos_run_plots_before_the_failure_summary(
    run_harness: Any, monkeypatch: pytest.MonkeyPatch
):
    plotted: list[Path] = []
    monkeypatch.setattr(awsb, "plot_throughput", lambda _, d: plotted.append(d))
    run = run_harness
    write_failed_run(run.run_dir)
    state: Any = SimpleNamespace(
        agent=None,
        run_dir=run.run_dir,
        error="node2 timed out",
        manifest=chaos_manifest(),
        fleet=SimpleNamespace(system=FakeSystem()),
    )
    assert awsb.report(state) is None
    assert plotted == [run.run_dir]
    assert "## Chaos test" in (run.run_dir / "summary.md").read_text()


def test_chaos_cell_counts_faults_and_timeouts():
    events = [*EVENTS, ev(430, "timeout", "node2", "wipe")]
    assert awsb.chaos_cell(CONFIG, events) == "2 faults, 1 timeouts"
    assert awsb.chaos_cell({"nodes": 3}, events) == "-"


def test_chaos_index_row_has_the_chaos_rate_and_the_mean_decided():
    manifest = chaos_manifest()
    result = make_result(steps_of([4.0, 2.0]))
    cells = awsb.index_cells(manifest, "01-run", result, 0, None, EVENTS)
    assert (cells["rate"], cells["decided"], cells["bound"]) == ("4", "3", "-")
    assert cells["chaos"] == "2 faults, 0 timeouts"
    failed = awsb.index_cells(manifest, "01-run", None, 3, None, EVENTS)
    assert (failed["rate"], failed["decided"], failed["status"]) == ("4", "-", "failed")
    plain = awsb.index_cells(index_manifest(), "01-run", None, 3, None)
    assert plain["chaos"] == "-"


def test_tx_counts_of_a_run_without_load_log_is_empty(tmp_path: Path):
    assert tx_count_total(tmp_path) == 0
    write_failed_run(tmp_path)
    (tmp_path / "load.jsonl").unlink()
    manifest = netbench.read_json(tmp_path / "manifest.json")
    awsb.write_failure_summary(tmp_path, manifest, "boom", fallback=True)
    text = (tmp_path / "summary.md").read_text()
    assert text.startswith("## Chaos test\n") and "- Txs:" not in text


def tx_count_total(run_dir: Path) -> int:
    return sum(awsb.tx_counts(run_dir).values())


def test_failure_summary_falls_back_when_the_chaos_report_raises(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    write_failed_run(tmp_path)

    def broken(*_: Any, **__: Any) -> Any:
        raise ValueError("bad chaos.jsonl")

    monkeypatch.setattr(awsb, "chaos_view", broken)
    manifest = netbench.read_json(tmp_path / "manifest.json")
    awsb.write_failure_summary(tmp_path, manifest, "gate failed", fallback=True)
    text = (tmp_path / "summary.md").read_text()
    assert text.startswith("## Network benchmark\n\n**invalid**: gate failed\n")
    assert "node0 log" in text and "node2 log" in text


def test_render_of_a_failed_chaos_run_raises_when_the_chaos_report_does(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    write_failed_run(tmp_path)
    monkeypatch.setattr(awsb, "chaos_view", raiser(ValueError("bad chaos.jsonl")))
    args = awsb.parse_args(["render", str(tmp_path)])
    with pytest.raises(ValueError, match="bad chaos.jsonl"):
        awsb.cmd_render(args, FakeSystem(run=FakeRunner()))


def test_report_of_a_failed_run_writes_its_error(
    run_harness: Any, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "plot_throughput", lambda *_: None)
    run = run_harness
    write_failed_run(run.run_dir)
    state: Any = SimpleNamespace(
        agent=None,
        run_dir=run.run_dir,
        error="node2 timed out",
        manifest=chaos_manifest(),
        fleet=SimpleNamespace(system=FakeSystem()),
    )
    awsb.report(state)
    assert (run.run_dir / "error.txt").read_text() == "node2 timed out"
    assert "error.txt" in awsb.PUBLISH_OPTIONAL


def test_render_of_a_failed_chaos_run_reads_back_its_error(tmp_path: Path):
    write_failed_run(tmp_path)
    (tmp_path / "chaos.jsonl").write_text("".join(json.dumps(e) + "\n" for e in EVENTS))
    (tmp_path / "error.txt").write_text("gate failed hard")
    args = awsb.parse_args(["render", str(tmp_path)])
    awsb.cmd_render(args, FakeSystem(run=FakeRunner()))
    assert "gate failed hard" in (tmp_path / "summary.md").read_text()


def test_totals_show_duplicates_only_when_non_zero():
    txs = Counter(included=8)
    rows = ch.fault_rows(EVENTS)
    meta = {"missing_payloads": [], "submit_failovers": 1, "submit_duplicates": 3}
    lines = awsb.chaos_totals(rows, EVENTS, T0 + 240, CONFIG, meta, txs)
    assert lines[1].endswith("submit failovers 1; duplicate inclusions 3")
