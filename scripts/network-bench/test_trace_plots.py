import json
from pathlib import Path

import pytest
from fakes import load_script
from test_leadertrace import MS, write_trace

plots = load_script("trace-plots")


def write_run(run_dir: Path, views: int = 40) -> None:
    """Three nodes rotating the lead, with the timings of `three_nodes` in every view."""
    rows: dict[int, list[tuple[int, str, int]]] = {0: [], 1: [], 2: []}
    for view in range(views):
        t0 = 10 * view
        leader = view % 3
        for event, offset in (
            ("request_block_header_queued", 2),
            ("header_created_applied", 5),
            ("proposal_queued", 10),
            ("leaf_decided", 40),
        ):
            rows[leader].append((view, event, (t0 + offset) * MS))
        for node, delay in zip(range(3), (12, 14, 16)):
            rows[node].append(
                (view, "cert1_v_minus_1_input_dispatched", (t0 + 10 + delay) * MS)
            )
            if node != leader:
                rows[node].append((view, "leaf_decided", (t0 + 40) * MS))
    for node, node_rows in rows.items():
        trace_dir = run_dir / "hosts" / f"node{node}" / "trace"
        trace_dir.mkdir(parents=True)
        write_trace(trace_dir / f"leader_trace_node{node}.csv", node, node_rows)


def test_main_writes_the_plots_and_stats(tmp_path: Path):
    write_run(tmp_path)
    plots.main([str(tmp_path)])
    out = tmp_path / "trace"
    for name in ("phases.png", "finality.png"):
        assert (out / name).read_bytes().startswith(b"\x89PNG")
    stats = json.loads((out / "stats.json").read_text())
    assert stats["phases"]["idle"] == {
        "median": pytest.approx(2),
        "p95": pytest.approx(2),
    }
    assert set(stats["phases"]) == set(plots.PHASES)
    assert stats["finality"]["p50"] == pytest.approx(30)
    assert stats["finality"]["skipped"] == {}
    assert stats["views"] == {"plotted": 30, "skipped": {}}


def test_main_exits_with_a_message_when_there_are_no_traces(tmp_path: Path):
    (tmp_path / "hosts" / "node0").mkdir(parents=True)
    with pytest.raises(SystemExit, match="no leader_trace_node"):
        plots.main([str(tmp_path)])
    assert not (tmp_path / "trace").exists()


def test_main_exits_when_no_view_is_plottable(tmp_path: Path):
    write_run(tmp_path, views=5)
    with pytest.raises(SystemExit, match="no views"):
        plots.main([str(tmp_path)])


def test_stats_report_finality_skips_by_reason(tmp_path: Path):
    write_run(tmp_path)
    csv = tmp_path / "hosts/node1/trace/leader_trace_node1.csv"
    csv.write_text(csv.read_text() + "500,1,leaf_decided,1\n")
    plots.main([str(tmp_path)])
    stats = json.loads((tmp_path / "trace" / "stats.json").read_text())
    assert stats["finality"]["skipped"] == {"no leader": 1}
