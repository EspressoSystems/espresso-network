import json
from pathlib import Path

import netbench
import numpy as np
import pytest
from fakes import load_script

plot = load_script("throughput-plot")

TX_SIZE = 1_000_000


def test_rates_from_counters_and_tx_times():
    t, rate = plot.decided_mb_s(
        [
            {"ts": 10.0, "decided_bytes": 0.0},
            {"ts": 11.0, "decided_bytes": 5e6},
            {"ts": 13.0, "decided_bytes": 9e6},
        ]
    )
    assert list(t) == [11.0, 13.0]
    assert list(rate) == [5.0, 2.0]

    t, rate = plot.binned_mb_s([10.1, 10.2, 10.9, 12.5], TX_SIZE, 10.0, 13.0)
    assert list(t) == [11.0, 12.0, 13.0]
    assert list(rate) == [3.0, 0.0, 1.0]


def test_client_lag_is_measured_from_the_validator_that_showed_the_last_block():
    clients = [
        {"end": 3, "t_done": 105.0, "bytes": 4_000_000, "status": "ok"},
        {"end": 4, "t_done": 106.0, "bytes": 0, "status": "missing"},
        {"end": 9, "t_done": 107.0, "bytes": 1_000_000, "status": "ok"},
    ]
    done, lag = plot.client_lag(clients, {2: 104.5, 3: 105.5})
    assert list(done) == [105.0]
    assert list(lag) == [500.0]
    t, rate = plot.binned_bytes_mb_s(
        np.array([c["t_done"] for c in clients]),
        np.array([c["bytes"] for c in clients]),
        104.0,
        108.0,
    )
    assert list(t) == [105.0, 106.0, 107.0, 108.0]
    assert list(rate) == [0.0, 4.0, 0.0, 1.0]


def test_block_series_over_the_trailing_window():
    counters = [{"ts": float(t), "decided_bytes": 4e6 * t} for t in range(25)]
    blocks = np.linspace(0.5, 24.5, 49)
    t, interval, size = plot.block_series(blocks, counters, w=10)
    assert t[0] == 10.0
    assert interval[0] == pytest.approx(0.5)
    assert size[0] == pytest.approx(2.0)
    t, interval, size = plot.block_series(np.array([]), counters, w=10)
    assert np.isnan(interval).all() and np.isnan(size).all()


def test_rolling_median_and_in_flight():
    at = np.array([1.0, 2.0, 3.0, 12.0])
    med = plot.rolling_median(
        at, np.array([10.0, 30.0, 20.0, 5.0]), np.array([5.0, 12.0, 30.0]), w=10
    )
    assert med[0] == 20.0 and med[1] == 12.5 and np.isnan(med[2])
    flight = plot.in_flight(
        np.array([0.0, 1.0, 2.0]), np.array([3.0, 1.5, 9.0]), np.array([1.0, 2.0, 5.0])
    )
    assert list(flight) == [2, 2, 1]


@pytest.mark.parametrize(
    ("text", "mb"), [("50mb", 50.0), ("1gb", 1000.0), ("2MiB", 2.097152)]
)
def test_max_block_mb(tmp_path: Path, text: str, mb: float):
    genesis = tmp_path / "genesis.toml"
    genesis.write_text(f'[chain_config]\nmax_block_size = "{text}"\n')
    assert plot.max_block_mb(genesis) == pytest.approx(mb)


def test_main_writes_the_chart_from_a_minimal_run_dir(tmp_path: Path):
    run_dir = tmp_path / "fleet" / "runs" / "01-run"
    run_dir.mkdir(parents=True)
    t0 = 1000.0
    consensus = [
        {"ts": t0 + i, "decided_bytes": 2e6 * i, "timeouts": int(i > 20)}
        for i in range(30)
    ]
    load = [
        {
            "t_submit": t0 + i / 2,
            "t_included": t0 + i / 2 + 1 if i % 5 else None,
            "height": i // 2 if i % 5 else None,
        }
        for i in range(40)
    ]
    heights = [
        {"height": h, "validator": t0 + h + 0.5, "query": t0 + h + 1} for h in range(20)
    ]
    steps = [
        {
            "rate_mb_s": rate,
            "refine": refine,
            "t_start": t0 + start,
            "t_mid": t0 + start + 7.5,
            "t_end": t0 + start + 15,
            "consensus_fails": [],
            "query_fails": ["lag"] if refine else [],
        }
        for rate, refine, start in ((2.0, False, 0), (3.0, True, 15))
    ]
    steps[1]["stopped_early"] = True
    config = {"tx_size": TX_SIZE, "tx_timeout_s": 60}
    for name, data in (("run.json", {"t0": t0}), ("config.json", config)):
        (run_dir / name).write_text(json.dumps(data))
    (run_dir / "steps.json").write_text(json.dumps(steps))
    (run_dir / "genesis.toml").write_text('[chain_config]\nmax_block_size = "50mb"\n')
    rows_by_name = (
        ("consensus.jsonl", consensus),
        ("load.jsonl", load),
        ("heights.jsonl", heights),
    )
    for name, rows in rows_by_name:
        (run_dir / name).write_text("".join(json.dumps(r) + "\n" for r in rows))
    plot.main([str(run_dir)])
    assert (run_dir / "throughput.png").read_bytes().startswith(b"\x89PNG")
    three_panels = (run_dir / "throughput.png").read_bytes()
    clients = [
        {
            "ns": 1,
            "reader": 0,
            "start": h,
            "end": h + 1,
            "t_start": t0 + h + 1,
            "t_done": t0 + h + 1.2,
            "bytes": 1_000_000,
            "status": "ok",
        }
        for h in range(18)
    ]
    (run_dir / "clients.jsonl").write_text(
        "".join(json.dumps(c) + "\n" for c in clients)
    )
    plot.main([str(run_dir)])
    assert (run_dir / "throughput.png").read_bytes() != three_panels
    # A run whose readers wrote nothing keeps the three panels.
    (run_dir / "clients.jsonl").write_text("")
    plot.main([str(run_dir)])
    assert (run_dir / "throughput.png").read_bytes() == three_panels


def fault_event(ts: float, event: str, node: str, kind: str = "kill") -> dict:
    return {"ts": ts, "event": event, "node": node, "kind": kind}


CHAOS_EVENTS = [
    fault_event(1005.0, "fault", "node5"),
    fault_event(1015.0, "started", "node5"),
    fault_event(1020.0, "rejoined", "node5"),
    fault_event(1030.0, "caught_up", "node5"),
    fault_event(1008.0, "fault", "node2", "wipe"),
    fault_event(1018.0, "rejoined", "node2"),
    fault_event(1040.0, "timeout", "node2"),
    fault_event(1050.0, "fault", "node7", "restart"),
]


def test_phases_of_a_killed_query_node_run_from_fault_to_catch_up():
    row = netbench.fault_rows(CHAOS_EVENTS)[0]
    assert plot.phase_segments(row, end=1100.0) == [
        ("down", 1005.0, 1015.0),
        ("recovering", 1015.0, 1020.0),
        ("catching up", 1020.0, 1030.0),
    ]


def test_a_node_without_a_start_event_has_no_down_phase():
    row = {"fault": 10.0, "rejoined": 25.0}
    assert plot.phase_segments(row, end=100.0) == [("recovering", 10.0, 25.0)]


def test_a_timeout_ends_the_open_phase_and_a_run_end_ends_an_unfinished_one():
    wiped = netbench.fault_rows(CHAOS_EVENTS)[1]
    assert plot.phase_segments(wiped, end=1100.0) == [
        ("recovering", 1008.0, 1018.0),
        ("catching up", 1018.0, 1040.0),
    ]
    open_ = netbench.fault_rows(CHAOS_EVENTS)[2]
    assert plot.phase_segments(open_, end=1100.0) == [("recovering", 1050.0, 1100.0)]


def write_run(run_dir: Path, steps_only: bool = False) -> float:
    """A minimal run dir for `main`. `steps_only`: as a run that failed leaves it, without
    `run.json` and with an empty `consensus.jsonl`."""
    t0 = 1000.0
    run_dir.mkdir(parents=True, exist_ok=True)
    consensus = (
        []
        if steps_only
        else [
            {"ts": t0 + i, "decided_bytes": 2e6 * i, "timeouts": 0} for i in range(60)
        ]
    )
    load = [
        {"t_submit": t0 + i, "t_included": t0 + i + 1, "height": i} for i in range(50)
    ]
    heights = [{"height": h, "validator": t0 + h + 0.5} for h in range(50)]
    step = {
        "rate_mb_s": 2.0,
        "refine": False,
        "t_start": t0,
        "t_mid": t0 + 30,
        "t_end": t0 + 60,
        "consensus_fails": [],
        "query_fails": [],
    }
    files = {
        "config.json": {"tx_size": TX_SIZE, "tx_timeout_s": 60},
        "steps.json": [step],
    }
    if not steps_only:
        files["run.json"] = {"t0": t0}
    for name, data in files.items():
        (run_dir / name).write_text(json.dumps(data))
    (run_dir / "genesis.toml").write_text('[chain_config]\nmax_block_size = "50mb"\n')
    for name, rows in (
        ("consensus.jsonl", consensus),
        ("load.jsonl", load),
        ("heights.jsonl", heights),
        ("chaos.jsonl", CHAOS_EVENTS),
    ):
        (run_dir / name).write_text("".join(json.dumps(r) + "\n" for r in rows))
    return t0


def test_main_draws_the_chaos_panel_and_shading(tmp_path: Path):
    run_dir = tmp_path / "fleet" / "runs" / "01-run"
    write_run(run_dir)
    plot.main([str(run_dir)])
    assert (run_dir / "throughput.png").read_bytes().startswith(b"\x89PNG")


def test_main_charts_a_failed_run_without_counters_or_run_json(tmp_path: Path):
    run_dir = tmp_path / "fleet" / "runs" / "01-run"
    write_run(run_dir, steps_only=True)
    plot.main([str(run_dir)])
    assert (run_dir / "throughput.png").read_bytes().startswith(b"\x89PNG")


def test_last_time_is_the_latest_step_submit_or_event():
    assert plot.last_time([{"t_end": 5.0}], [{"t_submit": 7.0}], [{"ts": 6.0}]) == 7.0


def test_main_draws_the_chaos_lanes_and_the_clients_panel_together(tmp_path: Path):
    run_dir = tmp_path / "fleet" / "runs" / "01-run"
    t0 = write_run(run_dir)
    plot.main([str(run_dir)])
    chaos_only = (run_dir / "throughput.png").read_bytes()
    clients = [
        {"end": h + 1, "t_done": t0 + h + 1.2, "bytes": 1_000_000, "status": "ok"}
        for h in range(18)
    ]
    (run_dir / "clients.jsonl").write_text(
        "".join(json.dumps(c) + "\n" for c in clients)
    )
    plot.main([str(run_dir)])
    both = (run_dir / "throughput.png").read_bytes()
    assert both.startswith(b"\x89PNG")
    assert both != chaos_only
