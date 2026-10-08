import json
from pathlib import Path

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
