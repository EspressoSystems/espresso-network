import json
from pathlib import Path

import leadertrace
import pytest
from fakes import load_script
from test_leadertrace import MS, leader_rows, write_trace

plots = load_script("trace-plots")


def write_run(
    run_dir: Path, views: int = 40, overrides: dict[tuple[int, str], int] | None = None
) -> None:
    """Three nodes rotating the lead, with the timings of `three_nodes` in every view.
    `overrides` maps (view, event) to a replacement ts_ns."""
    overrides = overrides or {}
    rows: dict[int, list[tuple[int, str, int]]] = {0: [], 1: [], 2: []}
    for view in range(views):
        rows[view % 3] += leader_rows(view)
        for node in set(range(3)) - {view % 3}:
            rows[node].append((view, "leaf_decided", (10 * view + 40) * MS))
    for node, node_rows in rows.items():
        trace_dir = run_dir / "hosts" / f"node{node}" / "trace"
        trace_dir.mkdir(parents=True)
        write_trace(
            trace_dir / f"leader_trace_node{node}.csv",
            node,
            [(v, e, overrides.get((v, e), ts)) for v, e, ts in node_rows],
        )


def write_steps(run_dir: Path) -> None:
    """View v has t0 at v * 10 ms: the measured halves hold views 10..19 and 21..30, and the
    last step holds none."""
    steps = [
        {
            "rate_mb_s": 120.0,
            "refine": False,
            "t_start": 0.0,
            "t_mid": 0.095,
            "t_end": 0.195,
        },
        {
            "rate_mb_s": 155.0,
            "refine": True,
            "t_start": 0.1,
            "t_mid": 0.205,
            "t_end": 0.305,
        },
        {
            "rate_mb_s": 200.0,
            "refine": False,
            "t_start": 9.0,
            "t_mid": 10.0,
            "t_end": 11.0,
        },
    ]
    (run_dir / "steps.json").write_text(json.dumps(steps))


def read_stats(run_dir: Path) -> dict:
    return json.loads((run_dir / "trace" / "stats.json").read_text())


def test_main_writes_the_plots_and_stats(tmp_path: Path):
    write_run(tmp_path)
    plots.main([str(tmp_path)])
    out = tmp_path / "trace"
    for name in (
        "leader_path.png",
        "leader_path_typical.png",
        "leader_path_worst.png",
        "finality.png",
    ):
        assert (out / name).read_bytes().startswith(b"\x89PNG")
    assert not (out / "phases.png").exists()
    stats = read_stats(tmp_path)
    path = stats["leader_path"]
    assert list(path["segments"]) == [n for n, _ in leadertrace.LEADER_PATH]
    assert path["segments"]["block build"] == {
        "median": pytest.approx(2),
        "p95": pytest.approx(2),
        "negative": 0,
    }
    assert path["total"] == {"median": pytest.approx(40), "p95": pytest.approx(40)}
    assert (path["views"], path["skipped"]) == (30, {})
    assert stats["finality"] == {
        "views": 30,
        "skipped": {},
        "median": pytest.approx(30),
        "p95": pytest.approx(30),
    }
    assert "steps" not in stats


def test_stats_count_negative_segments_of_a_view(tmp_path: Path):
    write_run(tmp_path, overrides={(13, "vote2_v_minus_1_broadcast_end"): 149 * MS})
    plots.main([str(tmp_path)])
    segments = read_stats(tmp_path)["leader_path"]["segments"]
    assert segments["Cert1 -> vote2 sent"]["negative"] == 1


def test_main_writes_per_step_stats_and_table(tmp_path: Path):
    write_run(tmp_path)
    write_steps(tmp_path)
    plots.main([str(tmp_path)])
    steps = read_stats(tmp_path)["steps"]
    assert [(s["rate_mb_s"], s["refine"], s["views"]) for s in steps] == [
        (120.0, False, 10),
        (155.0, True, 10),
        (200.0, False, 0),
    ]
    assert steps[0]["segments"]["header"] == pytest.approx(1)
    assert steps[0]["total"] == pytest.approx(40)
    assert steps[2]["segments"] is None
    assert steps[2]["total"] is None
    lines = (tmp_path / "trace" / "leader_path.md").read_text().splitlines()
    names = [n for n, _ in leadertrace.LEADER_PATH]
    assert lines[0] == "| load | views | " + " | ".join(names) + " | total |"
    assert lines[2].startswith("| 120 MB/s | 10 | 2.0 | 2.0 | 1.0 |")
    assert lines[2].endswith("| 13.0 | 40.0 |")
    assert lines[3].startswith("| 155 MB/s (refine) | 10 |")
    assert lines[4] == "| 200 MB/s | 0 | " + " | ".join(["-"] * (len(names) + 1)) + " |"
    assert len(lines) == 5


def test_stats_cover_only_the_measured_halves_with_steps(tmp_path: Path):
    write_run(tmp_path)
    write_steps(tmp_path)
    plots.main([str(tmp_path)])
    stats = read_stats(tmp_path)
    assert stats["leader_path"]["views"] == 20
    assert stats["finality"]["views"] == 19  # view 9 is before the warmup


def test_table_without_steps_has_one_row_for_all_views(tmp_path: Path):
    write_run(tmp_path)
    plots.main([str(tmp_path)])
    lines = (tmp_path / "trace" / "leader_path.md").read_text().splitlines()
    assert len(lines) == 3
    assert lines[2].startswith("| all views | 30 | 2.0 |")


def test_step_stats_bin_views_by_t0_in_the_measured_half():
    path = {2: [9.0] * 13, 3: [1.0] * 13, 4: [3.0] * 13, 5: [9.0] * 13}
    times = {1: 10.0, 2: 20.0, 3: 30.0, 4: 40.0, 5: 50.0}
    steps = [
        {
            "rate_mb_s": 1.0,
            "refine": False,
            "t_start": 0.0,
            "t_mid": 15.0,
            "t_end": 30.0,
        }
    ]
    (step,) = plots.step_stats(path, times, steps)
    assert step["views"] == 2
    assert set(step["segments"].values()) == {2.0}
    assert step["total"] == 26.0


def test_pick_views_takes_the_median_and_the_maximum():
    totals = {1: 10.0, 2: 50.0, 3: 12.0, 4: 11.0, 5: 90.0}
    assert plots.pick_views(totals) == {"typical": 3, "worst": 5}


def test_pick_views_with_a_single_view():
    assert plots.pick_views({7: 3.0}) == {"typical": 7, "worst": 7}


def test_pick_views_prefers_the_first_view_on_ties():
    assert plots.pick_views({1: 5.0, 2: 5.0, 3: 5.0}) == {"typical": 1, "worst": 1}


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
    assert read_stats(tmp_path)["finality"]["skipped"] == {"no leader": 1}


def test_trace_hosts_maps_trace_node_ids_to_the_host_dirs():
    paths = [
        Path("run/hosts/node0/trace/leader_trace_node4.csv"),
        Path("run/hosts/node2/trace/leader_trace_node0.csv"),
    ]
    assert plots.trace_hosts(paths) == {4: "node0", 0: "node2"}
