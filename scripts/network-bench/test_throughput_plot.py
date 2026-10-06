import json
from pathlib import Path

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


def test_main_writes_the_chart_from_a_minimal_run_dir(tmp_path: Path):
    run_dir = tmp_path / "fleet" / "runs" / "01-run"
    run_dir.mkdir(parents=True)
    t0 = 1000.0
    consensus = [{"ts": t0 + i, "decided_bytes": 2e6 * i} for i in range(30)]
    load = [
        {"t_submit": t0 + i / 2, "t_included": t0 + i / 2 + 1 if i % 5 else None}
        for i in range(40)
    ]
    steps = [
        {
            "rate_mb_s": rate,
            "refine": refine,
            "t_start": t0 + start,
            "t_end": t0 + start + 15,
            "consensus_fails": [],
            "query_fails": ["lag"] if refine else [],
        }
        for rate, refine, start in ((2.0, False, 0), (3.0, True, 15))
    ]
    for name, data in (("run.json", {"t0": t0}), ("config.json", {"tx_size": TX_SIZE})):
        (run_dir / name).write_text(json.dumps(data))
    (run_dir / "steps.json").write_text(json.dumps(steps))
    for name, rows in (("consensus.jsonl", consensus), ("load.jsonl", load)):
        (run_dir / name).write_text("".join(json.dumps(r) + "\n" for r in rows))
    plot.main([str(run_dir)])
    assert (run_dir / "throughput.png").read_bytes().startswith(b"\x89PNG")
