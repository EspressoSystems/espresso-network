import asyncio
import base64
import collections
import dataclasses
import functools
import inspect
import itertools
import json
import logging
import math
import statistics
import threading
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any, Literal, TypeVar, cast

import fakes
import netbench
import pytest
from fakes import TOPOLOGY, make_result, quantiles, step, write_run_dir

_T = TypeVar("_T")

# The measured half of a step holds STEP_S / 2 counter samples, COUNTER_POLL_S apart; fewer
# make the decided rate too noisy for the ramp verdicts.
STEP_S = 8.0

ALL_ANSWERED = {n: 1.0 for n in TOPOLOGY["nodes"]}


def some(x: _T | None) -> _T:
    assert x is not None
    return x


def deployment() -> netbench.DeploymentMeta:
    return {
        "provider": "aws",
        "account": "027574771971",
        "region": "eu-west-1",
        "az": "eu-west-1b",
        "ami": "ami-0abc",
        "hosts": {
            "ctl": {"instance_type": "c8g.2xlarge"},
            "node0": {"instance_type": "c8g.4xlarge"},
        },
        "images": {"espresso-node": "release-x@sha256:1a2b"},
        "image_revision": "bd2ad6e1dc7",
        "start_spread_s": 0.8,
        "clock_offset_ms_max": 12.0,
        "cost_usd": {"expected": 2.4, "bound": 4.6},
    }


def compare(
    current: netbench.BenchResult,
    runs: list[netbench.BenchResult],
    error: str | None = None,
    source: Literal["main", "reference"] = "main",
) -> netbench.Comparison:
    return netbench.compare(current, {"runs": runs, "error": error, "source": source})


def row(comparison, label):
    return next(r for r in comparison["capacity"] if r["label"] == label)


def step_row(comparison, rate, label):
    by_rate = next(c for c in comparison["steps"] if c["rate_mb_s"] == rate)
    return next(r for r in by_rate["rows"] if r["label"] == label)


def test_step_table_with_deltas():
    baseline = [make_result() for _ in range(5)]
    current = make_result(
        [step(4.0), step(6.0, consensus_p50=1200.0), step(8.0, 7.0, consensus=["x"])]
    )
    summary = netbench.render(current, compare(current, baseline))
    assert "| main median (n=5) |" in summary
    assert (
        "| 6 | 6 MB/s | **1200 ms (+33%)** | 2000 ms | 200 ms | 1200 ms | 0.5 s/MB "
        "| pass | 5 |"
    ) in summary
    assert "Test CPU" in summary


def test_rates_not_reached():
    current = make_result([step(4.0), step(6.0, 5.0, consensus=["x"]), step(5.0)])
    comparison = compare(current, [make_result()] * 3)
    eight = next(c for c in comparison["steps"] if c["rate_mb_s"] == 8.0)
    assert eight["n"] == 3
    summary = netbench.render(current, comparison)
    assert "| 8 | | | | | | | **not reached** | 3 |" in summary
    assert "| 5 | 5 MB/s | 900 ms |" in summary


def test_baseline_only_refine_rates_are_left_out():
    refine = step(7.0)
    refine["refine"] = True
    baseline = make_result([step(4.0), step(6.0), step(8.0, 6.0, ["x"]), refine])
    summary = netbench.render(make_result(), compare(make_result(), [baseline] * 3))
    assert not any(line.startswith("| 7 ") for line in summary.splitlines())


@pytest.mark.parametrize(
    ("baseline", "line", "absent"),
    [
        (([], None), "Baseline: no main runs to compare against.", None),
        (None, "Baseline: none given, no main runs to compare against.", None),
        (
            ([], "OSError: timed out"),
            "Baseline: fetching main runs failed: OSError: timed out",
            "no main runs",
        ),
    ],
)
def test_baseline_line(baseline, line, absent):
    current = make_result()
    comparison = None if baseline is None else compare(current, *baseline)
    summary = netbench.render(current, comparison)
    assert line in summary
    assert absent is None or absent not in summary


def test_deployment_and_hosts_sections():
    current = make_result()
    current["deployment"] = deployment()
    host: Any = fakes.host_sample(util=0.4, steal=0.5)
    hosts: Any = {"ctl": host, "node0": host | {"net_mb_s": 20.0}}
    current["hosts"] = hosts
    summary = netbench.render(current, None)
    for line in (
        "<details><summary>Deployment</summary>",
        "- account 027574771971 (eu-west-1), az eu-west-1b, ami ami-0abc",
        "- images @ bd2ad6e1dc7: espresso-node release-x@sha256:1a2b",
        "- instance types: ctl c8g.2xlarge, node0 c8g.4xlarge",
        "- cost: expected $2.40, bound $4.60",
        "<details><summary>Hosts</summary>",
        "| ctl | 40% | 0.5% | 1 | 1 |",
        "| node0 | 40% | 0.5% | 1 | 20 |",
    ):
        assert line in summary


def test_deployment_shows_the_max_block_size_when_recorded():
    current = make_result()
    current["deployment"] = deployment()
    assert "max block size" not in netbench.render(current, None)
    current["deployment"]["max_block_size"] = "30mb"
    assert "- max block size: 30mb" in netbench.render(current, None)


def test_load_baseline_single_result(tmp_path: Path):
    path = tmp_path / "result.json"
    netbench.write_json(path, make_result())
    baseline = netbench.load_baseline(path)
    assert (len(baseline["runs"]), baseline["source"]) == (1, "reference")
    netbench.write_json(path, {"runs": []})
    assert netbench.load_baseline(path)["source"] == "main"
    path.write_text("[]")
    with pytest.raises(ValueError):
        netbench.load_baseline(path)


FAIL = ["x"]
HIGHER = [step(4.0), step(6.0), step(8.0), step(10.0, 5.0, FAIL)]
BELOW = [step(4.0, 1.0, FAIL)]


@pytest.mark.parametrize(
    ("current", "steal", "runs", "expected"),
    [
        ([step(4.0), step(6.0, 5.8), step(8.0, 7.0, FAIL)], 0.0, [None], "same"),
        ([step(4.0), step(6.0, 5.0, FAIL), step(5.0)], 0.0, [None] * 3, "worse"),
        (BELOW, 0.0, [None] * 3, "worse"),
        ([step(4.0), step(6.0)], 0.0, [HIGHER] * 3, "inconclusive"),
        (None, 0.0, [BELOW, BELOW, None], "better"),
        (
            [step(4.0), step(6.0), step(8.0, 6.0, FAIL), step(7.0)],
            0.0,
            [None],
            "better",
        ),
        (None, 0.0, [None] * 3, "same"),
        ([step(4.0), step(6.0, 1.0, FAIL)], 8.0, [None], "inconclusive"),
    ],
)
def test_capacity_verdict(current, steal, runs, expected):
    comparison = compare(make_result(current, steal), [make_result(r) for r in runs])
    assert row(comparison, "capacity")["verdict"] == expected


def test_one_refine_step_shift_is_not_lost_to_float_error():
    resolution = netbench.ramp_resolution((0.1, 0.3))
    current: netbench.Limit = {"mb_s": 0.2, "bounded": True, "failed_at_mb_s": 0.25}
    baseline: netbench.Limit = {"mb_s": 0.3, "bounded": True, "failed_at_mb_s": 0.35}
    verdict = netbench.compare_capacity("x", current, [baseline], resolution, False)
    assert verdict["verdict"] == "worse"


def test_steps_compare_only_against_runs_at_that_rate():
    short = make_result([step(4.0, consensus_p50=500.0), step(6.0, 3.0, FAIL)])
    comparison = compare(make_result(), [short, make_result()])
    assert step_row(comparison, 4.0, "consensus p50")["baseline"] == 700.0
    eight = next(c for c in comparison["steps"] if c["rate_mb_s"] == 8.0)
    assert eight["n"] == 1


def test_spread_widens_threshold():
    history = [
        make_result([step(4.0, consensus_p50=v)]) for v in (600, 900, 1200, 750, 1050)
    ]
    current = make_result([step(4.0, consensus_p50=1200.0)])
    verdict = step_row(compare(current, history), 4.0, "consensus p50")
    assert verdict["threshold_pct"] > 15
    assert verdict["verdict"] == "same"


def edited(edits: dict[str, Any]) -> netbench.BenchResult:
    """`make_result()` with each dotted path in `edits` set to its value."""
    result = make_result()
    for path, value in edits.items():
        *parents, leaf = path.split(".")
        target: Any = result
        for key in parents:
            target = target[key]
        target[leaf] = value
    return result


CALIBRATION = "calibration.before.sha256_1t_mb_s"
INVALID = {"valid": False, "noisy": False, "reasons": ["x"]}
STALLED = {"stake_table": ["0x1", "0x2", "0x1"], "nodes.node2.decided_blocks": 0}
TRACKER_LAG = {"load.tracker_lag_ms": quantiles(300.0, 1500.0)}


@pytest.mark.parametrize(
    ("runs", "excluded"),
    [
        (
            [make_result(steal=8.0), edited({"validity": INVALID}), edited({})],
            {"noisy": 1, "invalid": 1},
        ),
        ([make_result(config_hash="other")], {"other config": 1}),
        (
            [edited({"runner.cpu_model": "x"}), edited({CALIBRATION: 1700.0})],
            {"other runner": 1, "other calibration": 1},
        ),
        ([edited({CALIBRATION: 1900.0})], {}),
        ([edited({"schema_version": 1})], {"other schema": 1}),
    ],
)
def test_baseline_exclusion(runs, excluded):
    comparison = compare(make_result(), [*runs, make_result()])
    assert comparison["excluded"] == excluded
    assert comparison["n"] == len(runs) + 1 - sum(excluded.values())


@pytest.mark.parametrize(
    ("result", "answered", "valid", "noisy", "reason"),
    [
        (make_result(steal=8.0), {}, True, True, "steal 8.0%"),
        (edited({"calibration.drift_pct": -15.0}), {}, True, True, "drift"),
        (edited(STALLED), {"node1": 0.5}, False, False, "stake table is not 3"),
        (edited(TRACKER_LAG), {}, True, True, "benchmark tracker behind"),
        (
            make_result([step(4.0), step(6.0, submitted=5.4), step(8.0, 7.0, FAIL)]),
            {},
            True,
            True,
            "load generator submitted 5.4 of 6",
        ),
        (
            make_result([step(4.0, 0.0, FAIL), step(6.0, 0.0, FAIL)]),
            {},
            False,
            False,
            "every step decided nothing",
        ),
    ],
)
def test_validity(result, answered, valid, noisy, reason):
    validity = netbench.check_validity(result, ALL_ANSWERED | answered)
    assert (validity["valid"], validity["noisy"]) == (valid, noisy)
    assert any(reason in r for r in validity["reasons"])


def test_a_stalled_node_and_a_partial_answer_add_their_own_reasons():
    validity = netbench.check_validity(edited(STALLED), ALL_ANSWERED | {"node1": 0.5})
    assert not validity["valid"]
    assert len(validity["reasons"]) == 3
    assert "node1 metrics answered for only 50% of the window" in validity["reasons"]
    assert "node2 decided no blocks in the window" in validity["reasons"]


class ScriptedPool:
    """An `Http` whose request answers come from `replies` in order, the last one repeating."""

    def __init__(self, clock: netbench.Clock, *replies: Any) -> None:
        self.clock = clock
        self.closed = threading.Event()
        self.replies = list(replies)
        self.calls = 0

    def request(
        self, method: str, url: str, body: bytes | None = None, timeout: float = 10.0
    ) -> tuple[int, bytes]:
        reply = self.replies[min(self.calls, len(self.replies) - 1)]
        self.calls += 1
        if isinstance(reply, Exception):
            raise reply
        return reply

    def close(self) -> None:
        self.closed.set()


def test_retries_failed_reads():
    clock = fakes.FakeClock()
    pool = ScriptedPool(clock, OSError("timed out"), (503, b""), (200, b"7"))
    assert netbench.get_ok(pool, "http://x") == b"7"
    assert clock.sleeps == [netbench.READ_RETRY_S] * 2


def test_gives_up_after_the_deadline():
    clock = fakes.FakeClock()
    pool = ScriptedPool(clock, OSError("timed out"))
    with pytest.raises(netbench.NetworkError):
        netbench.get_ok(pool, "http://x")
    assert pool.calls == math.ceil(netbench.READ_DEADLINE_S / netbench.READ_RETRY_S) + 1
    assert clock.time() >= netbench.READ_DEADLINE_S


def test_closing_the_pool_stops_retries(caplog: pytest.LogCaptureFixture):
    closing_at = 4 * netbench.READ_RETRY_S

    def close_late(now: float) -> None:
        if now >= closing_at:
            pool.close()

    clock = fakes.FakeClock(on_advance=close_late)
    pool = ScriptedPool(clock, OSError("timed out"))
    with (
        caplog.at_level(logging.DEBUG, netbench.log.name),
        pytest.raises(netbench.NetworkError),
    ):
        netbench.get_ok(pool, "http://x")
    assert clock.time() == closing_at
    levels = [record.levelname for record in caplog.records]
    assert levels[0] == "WARNING"
    assert set(levels[1:]) == {"DEBUG"}


def test_not_found_is_not_retried():
    clock = fakes.FakeClock()
    pool = ScriptedPool(clock, (404, b""), (500, b""))
    assert netbench.block_payload(pool, ["http://x"], 3) is None
    assert pool.calls == 1


@dataclasses.dataclass
class Load:
    node: fakes.FakeNode
    txs: list[dict[str, Any]]
    meta: dict[str, Any]
    steps: list[dict[str, Any]]
    heights: list[dict[str, Any]]


NODE_KEYS = frozenset(inspect.signature(fakes.FakeNode).parameters) - {"clock"}


def run_load(
    out: Path,
    duration: float,
    rate=0.02,
    cap_txs=1000,
    nodes=1,
    tx_size=1000,
    **kwargs: Any,
) -> Load:
    """One step of `duration` clock seconds at `rate` MB/s with at most `cap_txs` in flight,
    no warmup, unless `kwargs` sets `steps`. `FakeNode` arguments in `kwargs` go to the node."""
    fake = {k: kwargs.pop(k) for k in NODE_KEYS & kwargs.keys()}
    search = kwargs.pop("search", None)
    clock = fakes.FakeClock(threaded=any(k.endswith("_delay") for k in fake))
    node = fakes.FakeNode(clock, **fake)
    defaults: dict[str, Any] = {
        "tx_size": tx_size,
        "workers": 3,
        "steps": (rate,),
        "step_s": duration,
        "warmup_s": 0,
        "cap_s": cap_txs * tx_size / (rate * 1e6),
    }
    config = netbench.BenchConfig(**defaults | kwargs)
    urls = [node.url] * nodes
    load = netbench.generate_load(
        config,
        urls,
        [node.url],
        [node.url] * 2,
        out,
        clock,
        node.connect,
        search=search,
    )
    clock.run(load)
    return Load(
        node,
        list(netbench.read_jsonl(out / "load.jsonl")),
        netbench.read_json(out / "load-meta.json"),
        netbench.read_json(out / "steps.json"),
        list(netbench.read_jsonl(out / "heights.jsonl")),
    )


@pytest.fixture
def load(tmp_path: Path) -> Callable[..., Load]:
    return functools.partial(run_load, tmp_path)


@pytest.fixture
def staircase(load: Callable[..., Load]) -> Callable[..., Load]:
    """1 tx of 4000 bytes per 50 ms block: 0.08 MB/s of capacity."""
    return functools.partial(load, STEP_S, include=True, block_txs=1, tx_size=4000)


def verdicts(steps: list[dict[str, Any]]) -> list[tuple[float, bool, bool]]:
    return [
        (s["rate_mb_s"], s["refine"], not s["consensus_fails"] + s["query_fails"])
        for s in steps
    ]


def included(txs: list[dict[str, Any]]) -> bool:
    return {tx["status"] for tx in txs} == {"included"}


def max_latency(txs: list[dict[str, Any]]) -> float:
    return max(tx["t_included"] - tx["t_submit"] for tx in txs)


def test_submits_at_the_offered_rate(load):
    """1000 byte txs at 0.02 MB/s: one every 50 ms, independent of inclusion."""
    run = load(1.0, include=True, cap_txs=100, tx_timeout_s=5)
    submits = run.node.submits
    assert len(submits) in range(19, 22)
    gaps = sorted(b - a for a, b in itertools.pairwise(submits))
    assert gaps[len(gaps) // 2] == pytest.approx(0.05, abs=0.01)
    assert included(run.txs)
    assert run.meta["cap_waits"] == 0


def test_round_robin_over_submit_nodes(load):
    run = load(0.5, include=True, nodes=3, submit_nodes=2, tx_timeout_s=5)
    assert [tx["node"] for tx in run.txs[:4]] == [0, 1, 0, 1]


def test_txs_carry_queue_and_response_times(load):
    run = load(1.0, include=True, accept_delay=0.2, workers=1, tx_timeout_s=5)
    assert run.txs
    for tx in run.txs:
        assert tx["t_queued"] <= tx["t_submit"] <= tx["t_done"]
    assert run.txs[-1]["t_submit"] - run.txs[-1]["t_queued"] > 0.1
    assert all(step["cap_waits"] == 0 for step in run.steps)


def test_post_tx_stamps_t_done_when_the_request_fails():
    class Failing(fakes.FakePool):
        def request(self, *args: Any, **kwargs: Any) -> Any:
            raise OSError("refused")

    clock = fakes.FakeClock()
    pool = Failing(fakes.FakeNode(clock, include=False), clock)
    state = netbench.LoadState()
    tx = netbench.Tx(id=0, node=0, t_queued=0.0)
    state.submitted(tx)
    assert netbench.post_tx(pool, state, tx, ["http://x"], lambda: b"") == 0
    assert tx.t_done is not None
    assert tx.t_done >= tx.t_submit


def test_cap_blocks_until_timeout(load):
    """Room under the cap frees only on timeout, the first 1 s after the first submit."""
    run = load(2.5, include=False, rate=1.0, cap_txs=4, tx_timeout_s=1)
    assert run.node.submits[4] - run.node.submits[0] > 0.9
    assert run.meta["max_in_flight"] == 4
    assert run.meta["cap_waits"] > 0
    assert some(run.steps[0]["cap_waits"]) > 0
    assert {tx["status"] for tx in run.txs} == {"timeout"}


@pytest.mark.parametrize(
    ("duration", "accept_delay", "cap_txs", "tx_timeout_s", "latency_s"),
    [
        pytest.param(1.0, 0.2, 100, 5, 1.0, id="latency-excludes-queue"),
        pytest.param(2.0, 0.3, 5, 1, 1.5, id="queued-submit-does-not-time-out"),
    ],
)
def test_queued_submits(load, duration, accept_delay, cap_txs, tx_timeout_s, latency_s):
    """One submit thread: submits queue behind each other for longer than `tx_timeout_s`;
    neither the queue nor the tracker behind it counts as latency or times out."""
    run = load(
        duration,
        include=True,
        accept_delay=accept_delay,
        workers=1,
        cap_txs=cap_txs,
        tx_timeout_s=tx_timeout_s,
    )
    assert len(run.txs) > 5
    assert included(run.txs)
    assert max_latency(run.txs) < latency_s
    assert run.meta["max_in_flight"] <= cap_txs


def test_heartbeat_runs_alongside_the_load_and_stays_out_of_its_records(load):
    run = load(
        2.0,
        include=True,
        accept_delay=0.0,
        heartbeat_tx_s=20.0,
        tx_size=10_000,
        rate=0.05,
        tx_timeout_s=5,
    )
    assert included(run.txs)
    assert len(run.node.submits) - len(run.txs) >= 40


def test_a_heartbeat_too_large_for_the_drain_is_refused():
    with pytest.raises(ValueError, match="heartbeat"):
        netbench.BenchConfig(tx_size=1000, heartbeat_tx_s=20.0)


def test_slow_payloads_do_not_delay_block_times(load):
    run = load(0.5, include=True, payload_delay=0.1, cap_txs=100, tx_timeout_s=10)
    assert len(run.txs) > 5
    assert included(run.txs)
    assert max_latency(run.txs) < 0.4


def test_heights_on_validators_and_query_node(load):
    run = load(0.5, include=True, query_lag=0.3, cap_txs=100, tx_timeout_s=5)
    assert included(run.txs)
    by_height = {h["height"]: h for h in run.heights}
    for tx in run.txs:
        h = by_height[tx["height"]]
        assert tx["t_included"] == h["query"]
        assert h["scanned"] >= h["query"]
    lags = [h["query"] - h["validator"] for h in run.heights if h["query"]]
    assert statistics.median(lags) == pytest.approx(0.3, abs=0.12)


def test_included_before_the_submit_returns(load):
    run = load(0.6, include=True, reply_delay=0.4, rate=0.005, tx_timeout_s=2)
    assert len(run.txs) > 2
    assert included(run.txs)


def test_lost_payload_does_not_stall_later_blocks(load):
    """Steady state is 6 in flight, 8 on a jittery run; a stall gains 20 per second."""
    duration = netbench.MISSING_PAYLOAD_S + 2 * fakes.BLOCK_S + 1.0
    run = load(
        duration, include=True, lost=frozenset({10}), cap_txs=12, tx_timeout_s=1.5
    )
    assert run.meta["missing_payloads"] == [10]
    assert run.meta["cap_waits"] == 0
    done = [tx for tx in run.txs if tx["status"] == "included"]
    assert len(run.txs) - len(done) <= 2
    assert max_latency(done) < 0.5


def test_late_payload_counts_from_its_block(load):
    run = load(1.5, include=True, late={10: 1.0}, cap_txs=8, tx_timeout_s=3)
    assert run.meta["missing_payloads"] == []
    assert run.meta["cap_waits"] == 0
    assert included(run.txs)
    assert max_latency(run.txs) < 0.7


def test_staircase_stops_at_the_first_failing_step_and_refines(staircase):
    run = staircase(steps=(0.02, 0.04, 0.16), tx_timeout_s=1)
    assert verdicts(run.steps) == [
        (0.02, False, True),
        (0.04, False, True),
        (0.16, False, False),
        (0.1, True, False),
    ]


def test_refine_starts_after_the_backlog_drained(staircase):
    """0.1 MB/s leaves 40 txs behind; 0.07 MB/s alone keeps up but drains that backlog only
    at 2.5 tx/s, adding latency over the 300 ms target."""
    run = staircase(steps=(0.04, 0.1), latency_target_ms=300, tx_timeout_s=10)
    assert verdicts(run.steps) == [
        (0.04, False, True),
        (0.1, False, False),
        (0.07000000000000001, True, True),
    ]
    assert run.meta["drain_s"] > 0.2
    assert not run.meta["refine_skipped"]


def test_keep_going_runs_every_step_then_waits_for_the_backlog(staircase):
    """Both steps fail and leave 15 txs behind."""
    run = staircase(steps=(0.1, 0.12), tx_timeout_s=10, keep_going=True)
    assert [
        (s["rate_mb_s"], s["refine"], bool(s["consensus_fails"])) for s in run.steps
    ] == [(0.1, False, True), (0.12, False, True)]
    assert run.meta["drain_s"] > 0.2
    assert not run.meta["refine_skipped"]


@pytest.mark.parametrize(("keep_going", "length_s"), [(False, 10.0), (True, 30.0)])
def test_a_step_falling_behind_stops_early(staircase, keep_going, length_s):
    """0.16 MB/s against 0.08 of capacity: decided is half of submitted from the start."""
    run = staircase(steps=(0.16,), step_s=30, tx_timeout_s=60, keep_going=keep_going)
    (step,) = run.steps
    assert step["t_end"] - step["t_start"] == pytest.approx(length_s, abs=1.1)
    assert step["consensus_fails"][0].startswith("decided")


def test_keep_up():
    txs = [netbench.Tx(id=i, node=0, t_queued=i, t_submit=i) for i in range(10)]
    counters = [{"ts": t, "decided_bytes": t * 500.0} for t in range(10)]
    assert netbench.keep_up(txs, counters, 1000, 0.0, 10.0) == pytest.approx(0.5)
    assert netbench.keep_up(txs, counters[:1], 1000, 0.0, 9.0) is None
    assert netbench.keep_up([], counters, 1000, 0.0, 9.0) is None


@pytest.mark.parametrize(
    ("steps", "keep_going", "refine_skipped"),
    [((0.02, 0.16), False, True), ((0.1, 0.12), True, False)],
)
def test_backlog_not_drained(
    staircase, monkeypatch: pytest.MonkeyPatch, steps, keep_going, refine_skipped
):
    async def undrained(*_: Any, **__: Any) -> None:
        return None

    monkeypatch.setattr(netbench, "drain", undrained)
    run = staircase(steps=steps, tx_timeout_s=1, keep_going=keep_going)
    assert [s["rate_mb_s"] for s in run.steps] == list(steps)
    assert run.meta["drain_s"] is None
    assert run.meta["refine_skipped"] == refine_skipped


def test_steps_survive_a_failure_in_the_last_drain(
    staircase, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    async def gone(*_: Any, **__: Any) -> None:
        raise OSError("gone")

    monkeypatch.setattr(netbench, "drain", gone)
    with pytest.raises(netbench.NetworkError):
        staircase(steps=(0.1, 0.12), tx_timeout_s=1, keep_going=True)
    steps = netbench.read_json(tmp_path / "steps.json")
    assert [s["rate_mb_s"] for s in steps] == [0.1, 0.12]


def test_step_ends_on_time_while_waiting_for_room(load):
    """Nothing is included: the cap fills at once and frees only on timeouts after 3 s."""
    (only,) = load(1.0, include=False, cap_txs=2, tx_timeout_s=3).steps
    assert only["t_end"] - only["t_start"] < 1.3


def test_dead_process_raises_immediately():
    clock = fakes.FakeClock()
    pool = ScriptedPool(clock, OSError("refused"))
    with pytest.raises(netbench.NetworkError):
        netbench.wait_ready(pool, {"node0": "http://x"}, 1, 5.0, lambda: False, clock)
    assert clock.sleeps == []


def test_writes_stake_table_and_final_metrics(tmp_path: Path):
    clock = fakes.FakeClock()
    node = fakes.FakeNode(clock, True)
    topo: netbench.Topology = {
        "nodes": {"node0": node.url, "node1": node.url},
        "roles": {"node0": "validator, query, sqlite", "node1": "validator, sqlite"},
        "query_node": "node0",
    }
    fast: dict[str, Any] = {
        "tx_size": 1000,
        "workers": 1,
        "steps": (0.02,),
        "step_s": 1,
        "warmup_s": 0,
    }
    config = netbench.BenchConfig(**fast, cap_s=1.0, submit_nodes=1, tx_timeout_s=5)
    t0, t1 = netbench.drive_load(
        config, topo, tmp_path, lambda: True, clock, node.connect
    )
    assert t1 > t0
    for name in ("stake-table.json", "final-node0.prom", "final-node1.prom"):
        assert (tmp_path / name).exists()


def test_dead_network_raises_without_starting_load(tmp_path: Path):
    with pytest.raises(netbench.NetworkError, match="before load"):
        netbench.drive_load(
            netbench.BenchConfig(),
            TOPOLOGY,
            tmp_path,
            lambda: False,
            fakes.FakeClock(),
            fakes.no_http_pool,
        )
    assert list(tmp_path.iterdir()) == []


def write_load_files(out: Path, state: netbench.LoadState) -> dict[str, str]:
    netbench.write_load_files(
        out,
        state,
        netbench.Heights(7),
        [],
        [],
        b"\x01",
        {"drain_s": None, "refine_skipped": False, "stop_reason": "ramp exhausted"},
    )
    return {p.name: p.read_text() for p in out.iterdir()}


def test_cut_short_load_keeps_raw_files_without_steps(tmp_path: Path):
    files = write_load_files(tmp_path, netbench.LoadState())
    assert sorted(files) == [
        "consensus.jsonl",
        "heights.jsonl",
        "load-meta.json",
        "load.jsonl",
    ]
    assert netbench.read_json(tmp_path / "load-meta.json")["start_height"] == 7


def test_txs_never_sent_are_left_out(tmp_path: Path):
    state = netbench.LoadState()
    state.txs = [
        netbench.Tx(id=0, node=0, t_queued=0.0, t_submit=1.0),
        netbench.Tx(id=1, node=0, t_queued=0.0),
    ]
    write_load_files(tmp_path, state)
    assert [tx["id"] for tx in netbench.read_jsonl(tmp_path / "load.jsonl")] == [0]


def analyze(out: Path) -> netbench.BenchResult:
    return netbench.analyze(out, netbench.BenchConfig(), TOPOLOGY)


def test_steps_and_capacity(tmp_path: Path):
    write_run_dir(tmp_path)
    result = analyze(tmp_path)
    one, two = result["steps"]
    assert some(one["decided_mb_s"]) == pytest.approx(1.0)
    assert one["timeouts"] == 0
    assert some(one["consensus_latency_ms"])["p50"] == pytest.approx(500.0)
    assert some(one["query_lag_ms"])["p50"] == pytest.approx(300.0)
    assert some(one["latency_ms"])["p50"] == pytest.approx(800.0)
    assert some(one["mean_view_ms"]) == pytest.approx(250.0)
    assert some(one["cpu_s_per_mb"]) == pytest.approx(1.5)
    assert one["node_cpu"] == {"node0": 0.5, "node1": 0.5, "node2": 0.5}
    assert one["postgres_cpu"] is None
    assert one["passed"]
    # 16 included and one timed out, of 1 MB each, in the 15 s measured half.
    assert one["submitted_mb_s"] == pytest.approx(17 / 15)
    assert two["consensus_fails"] == ["decided 50% of submitted"]
    assert result["capacity"]["overall"] == {
        "mb_s": 1.0,
        "bounded": True,
        "failed_at_mb_s": 2.0,
    }
    node0, load = result["nodes"]["node0"], result["load"]
    assert (node0["decided_blocks"], node0["cpu_cores"]) == (120, 0.5)
    window = result["window"]
    assert (window["height_start"], window["height_end"]) == (200, 320)
    assert result["host"]["util_mean"] == pytest.approx(0.5)
    assert result["processes"]["node0"]["cpu_cores_mean"] == pytest.approx(0.5)
    assert (load["submitted"], load["included"], load["timeouts"]) == (42, 41, 1)
    assert some(load["tracker_lag_ms"])["p99"] == pytest.approx(100.0)
    assert result["validity"] == {"valid": True, "noisy": False, "reasons": []}


def test_result_json_is_finite(tmp_path: Path):
    write_run_dir(tmp_path)
    netbench.write_json(tmp_path / "result.json", analyze(tmp_path))
    with pytest.raises(ValueError):
        netbench.write_json(tmp_path / "bad.json", {"x": math.inf})


def test_report_keeps_the_ramp_verdict(tmp_path: Path):
    write_run_dir(tmp_path)
    path = tmp_path / "steps.json"
    steps = netbench.read_json(path)
    steps[0]["consensus_fails"] = ["decided 10% of offered"]
    netbench.write_json(path, steps)
    result = analyze(tmp_path)
    assert not result["steps"][0]["passed"]
    assert result["capacity"]["overall"] == {
        "mb_s": None,
        "bounded": True,
        "failed_at_mb_s": 1.0,
    }


def test_no_scrapes_is_invalid_not_a_crash(tmp_path: Path):
    write_run_dir(tmp_path)
    netbench.write_jsonl(
        tmp_path / "metrics.jsonl",
        (
            {"ts": ts, "node": node, "ok": False}
            for ts in range(90, 175, 5)
            for node in TOPOLOGY["nodes"]
        ),
    )
    result = analyze(tmp_path)
    assert not result["validity"]["valid"]
    assert result["steps"][0]["mean_view_ms"] is None
    assert "node0 metrics answered for only 0%" in result["validity"]["reasons"][0]
    assert "run **invalid**" in netbench.render(result, None)


@pytest.mark.parametrize(
    ("ramp", "passed", "keep_going", "expected"),
    [
        ((4.0, 6.0, 8.0), [], False, 4.0),
        ((4.0, 6.0, 8.0), [True], False, 6.0),
        ((4.0, 6.0, 8.0), [True, False], False, 5.0),
        ((4.0, 6.0, 8.0), [True, False, True], False, None),
        ((4.0, 6.0, 8.0), [True, True, True], False, None),
        ((4.0, 6.0), [False], False, None),
        ((4.0, 6.0), [False], True, 6.0),
        ((4.0, 6.0), [False, True], True, None),
    ],
)
def test_next_rate(ramp, passed, keep_going, expected):
    assert netbench.next_rate(ramp, passed, keep_going=keep_going) == expected


def idle_counters() -> list[dict[str, Any]]:
    """Decided bytes flat over the last DRAIN_IDLE_S, sampled every second up to t=0."""
    return [
        {"ts": -netbench.DRAIN_IDLE_S + i, "decided_bytes": 1}
        for i in range(int(netbench.DRAIN_IDLE_S) + 1)
    ]


def heights_at(validator: int, query: int | None) -> netbench.Heights:
    heights = netbench.Heights(0)
    heights.saw("validator", validator, -1.0)
    if query is not None:
        heights.saw("query", query, -1.0)
    return heights


def drain(
    state, heights, clock, decided=lambda ts: 1, max_s=netbench.DRAIN_MAX_S
) -> float | None:
    """Counters are sampled every second as the clock advances; `decided(ts)` is the value.
    Transactions of 2 bytes: a growth of 1 byte is consensus at work."""
    counters = idle_counters()
    chained = clock.on_advance

    def advance(now: float) -> None:
        while counters[-1]["ts"] + 1 <= now:
            ts = counters[-1]["ts"] + 1
            counters.append({"ts": ts, "decided_bytes": decided(ts)})
        if chained:
            chained(now)

    clock.on_advance = advance
    return clock.run(netbench.drain(state, counters, heights, clock, 2, max_s))


def sent_tx(state: netbench.LoadState, tx_id: int, sent: bool) -> netbench.Tx:
    tx = netbench.Tx(id=tx_id, node=0, t_queued=0.0)
    if sent:
        tx.t_submit = 0.5
    state.submitted(tx)
    return tx


def test_drain_drops_transactions_never_sent():
    class NoRequests(fakes.FakePool):
        def request(self, *args: Any, **kwargs: Any) -> Any:
            raise AssertionError("a dropped transaction was sent")

    clock = fakes.FakeClock()
    state = netbench.LoadState()
    sent, queued = sent_tx(state, 0, True), sent_tx(state, 1, False)
    drain(state, heights_at(5, 8), clock)
    assert state.pending == {0: sent}
    assert state.txs == [sent]
    pool = NoRequests(fakes.FakeNode(clock, include=False), clock)
    assert netbench.post_tx(pool, state, queued, "http://x", lambda: b"") is None
    assert queued.t_submit == math.inf


def test_drain_returns_after_the_grace_with_a_sent_transaction_pending():
    state = netbench.LoadState()
    sent_tx(state, 0, True)
    clock = fakes.FakeClock()
    elapsed = drain(state, heights_at(5, 8), clock)
    assert elapsed == pytest.approx(netbench.DRAIN_GRACE_S, abs=0.5)


def test_drain_gives_up_when_the_validator_height_stalls(caplog):
    clock = fakes.FakeClock()
    assert drain(netbench.LoadState(), heights_at(5, 5), clock) is None
    assert clock.now == pytest.approx(netbench.DRAIN_STALL_S, abs=0.5)
    assert "stalled" in caplog.text


@pytest.mark.parametrize(
    ("drain_s", "keep_going", "lines"),
    [
        (None, False, []),
        (3.0, False, ["- backlog drained in 3.0 s before the refine step"]),
        (3.0, True, ["- backlog drained in 3.0 s after the last step"]),
        (
            None,
            True,
            [
                "- backlog did not drain (chain stalled or cap of 600 s) after the last step"
            ],
        ),
    ],
)
def test_drain_lines(drain_s, keep_going, lines):
    assert netbench.drain_lines(drain_s, keep_going, 600.0) == lines


def test_theil_sen_ignores_an_outlier():
    points = [(float(x), 2.0 * x) for x in range(10)] + [(10.0, 100.0)]
    assert netbench.theil_sen(points) == pytest.approx(2.0)
    assert netbench.theil_sen([(1.0, 1.0)]) is None


def test_pace_follows_the_rate():
    assert netbench.tx_interval_s(1_000_000, 20.0) == pytest.approx(0.05)
    assert netbench.step_cap(netbench.BenchConfig(), 7.0) == 35


class LateClock(fakes.FakeClock):
    """Wakes come `late_s` after their time, the first `late_wakes` of them, as from an overloaded
    event loop."""

    def __init__(self, late_s: float, late_wakes: int) -> None:
        super().__init__()
        self.late_s = late_s
        self.late_wakes = late_wakes

    async def asleep(self, s: float) -> None:
        if s > 0 and self.late_wakes:
            self.late_wakes -= 1
            s += self.late_s
        await super().asleep(s)


def pace_for(
    monkeypatch: pytest.MonkeyPatch, clock: fakes.FakeClock, seconds: float
) -> list[netbench.Tx]:
    """4 MB/s of 1 MB txs, one every 0.25 s, with submits that do nothing."""

    async def submit(load: Any, tx: Any) -> None:
        pass

    monkeypatch.setattr(netbench, "submit_tx", submit)
    cfg = netbench.BenchConfig(tx_size=1_000_000, cap_s=1000.0)
    state = netbench.LoadState()
    load = netbench.Load(cfg, state, cast("Any", None), ["u"], cast("Any", None), clock)

    async def main() -> None:
        async with asyncio.TaskGroup() as submits:
            await netbench.pace(load, submits, 4.0, clock.time() + seconds)

    clock.run(main())
    return state.txs


def test_a_late_pacer_catches_up_instead_of_losing_rate(monkeypatch):
    on_time = pace_for(monkeypatch, LateClock(0.0, 0), 10.0)
    late = pace_for(monkeypatch, LateClock(0.5, 1000), 10.0)
    assert len(on_time) == pytest.approx(40, abs=1)
    assert len(late) == pytest.approx(40, abs=1)


def test_catching_up_is_bounded_to_a_second_of_load(monkeypatch):
    """A 5 s stall costs the schedule beyond CATCHUP_S, and the rest goes out at once."""
    txs = pace_for(monkeypatch, LateClock(5.0, 1), 10.0)
    burst = max(collections.Counter(tx.t_queued for tx in txs).values())
    assert burst == pytest.approx(netbench.CATCHUP_S * 4 + 1, abs=1)
    assert len(txs) < 40


@pytest.mark.parametrize("tx_size", [1000, 1001, 1002, 1_000_000])
def test_tx_bodies_are_exact_distinct_slices_of_the_pool(tx_size):
    cfg = netbench.BenchConfig(tx_size=tx_size)
    marker = bytes(range(16))
    bodies = netbench.TxBodies(cfg, marker)
    pool = base64.b64decode(bodies.encoded)
    seen = set()
    for tx_id in range(20):
        request = json.loads(bodies.request(tx_id, 10003))
        assert request["namespace"] == 10003
        payload = base64.b64decode(request["payload"], validate=True)
        assert len(payload) == tx_size
        assert payload[:24] == marker + tx_id.to_bytes(8, "big")
        content = payload[24 : 24 + bodies.groups * 3]
        assert content in pool
        seen.add(content)
    assert len(seen) == 20


def test_tx_bodies_reject_a_tx_smaller_than_its_header():
    with pytest.raises(ValueError, match="below the 24 B header"):
        netbench.TxBodies(netbench.BenchConfig(tx_size=23), bytes(16))
    with pytest.raises(ValueError, match="not a multiple of 3"):
        netbench.TxBodies(netbench.BenchConfig(), bytes(15))


def test_a_missing_payload_in_a_scan_batch_is_retried_and_included_later():
    clock = fakes.FakeClock()
    state, heights = netbench.LoadState(), netbench.Heights(0)
    heights.saw("query", 6, 0.0)
    for i in range(5):
        state.submitted(netbench.Tx(id=i, node=0, t_queued=0.0, t_submit=0.0))
    calls: collections.Counter[int] = collections.Counter()

    async def scan(height: int) -> list[int] | None:
        calls[height] += 1
        return None if height == 1 and calls[1] == 1 else [height]

    done = asyncio.Event()

    async def main() -> None:
        tracker = asyncio.create_task(
            netbench.track_inclusion(
                netbench.BenchConfig(), state, scan, heights, [], done, clock
            )
        )
        await clock.asleep(1.0)
        done.set()
        await tracker

    clock.run(main())
    assert calls[1] == 2
    assert [tx.status for tx in state.txs] == ["included"] * 5
    assert sorted(heights.scanned) == [0, 1, 2, 3, 4]
    assert not state.missing_payloads


def test_window_without_samples_has_zero_heights():
    series: dict[str, list[Any]] = {node: [] for node in TOPOLOGY["nodes"]}
    assert netbench.window(series, ["node0"], 1.0, 2.0) == {
        "t0": 1.0,
        "t1": 2.0,
        "height_start": 0,
        "height_end": 0,
    }


def test_keep_going_changes_the_config_hash():
    cfg = netbench.BenchConfig()
    assert netbench.config_hash(
        dataclasses.replace(cfg, keep_going=True), []
    ) != netbench.config_hash(cfg, [])


def test_idle_needs_decided_bytes_flat_for_longer_than_a_slow_block():
    """Blocks of 2.5 s under a backlog leave two equal samples 1 s apart while consensus is
    busy (run lulu-20260930-1758 reported "drained in 0.0 s")."""
    assert netbench.is_idle(idle_counters(), 0.0, 2)
    short = [c for c in idle_counters() if c["ts"] >= -2.0]
    assert not netbench.is_idle(short, 0.0, 2)
    busy = [{**c, "decided_bytes": 1 + (c["ts"] > -3.0)} for c in idle_counters()]
    assert not netbench.is_idle(busy, 0.0, 2)
    assert not netbench.is_idle([], 0.0, 2)


def test_idle_allows_heartbeat_bytes():
    """50 heartbeats a second of 8 bytes plus a 4-byte tx table entry, 1 MB transactions."""
    beating = [{**c, "decided_bytes": 600 * (c["ts"] + 10)} for c in idle_counters()]
    assert netbench.is_idle(beating, 0.0, 1_000_000)
    one_tx = [{**c, "decided_bytes": 1e6 * (c["ts"] > -3.0)} for c in idle_counters()]
    assert not netbench.is_idle(one_tx, 0.0, 1_000_000)


class BeatPool:
    """Records heartbeat bodies; sets `stop` after `beats` requests, refuses those to `down`."""

    def __init__(
        self, beats: int, stop: threading.Event, down: tuple[str, ...] = ()
    ) -> None:
        self.clock = fakes.FakeClock()
        self.beats, self.stop, self.down = beats, stop, down
        self.sent: list[tuple[float, str, bytes]] = []

    def request(
        self, method: str, url: str, body: bytes | None = None
    ) -> tuple[int, bytes]:
        assert method == "POST" and body is not None
        self.sent.append((self.clock.time(), url, body))
        if len(self.sent) == self.beats:
            self.stop.set()
        if url.startswith(self.down):
            raise OSError("refused")
        return 200, b""


def test_heartbeat_sends_distinct_small_transactions_round_robin():
    stop = threading.Event()
    pool = BeatPool(6, stop)
    failed = netbench.heartbeat(
        cast("Any", pool), ["http://a", "http://b"], 7, 50.0, stop
    )
    assert failed == 0
    times = [t for t, _, _ in pool.sent]
    assert times == pytest.approx([i * 0.02 for i in range(6)])
    assert [url for _, url, _ in pool.sent] == [
        "http://a/v1/submit/submit",
        "http://b/v1/submit/submit",
    ] * 3
    payloads = [json.loads(body)["payload"] for _, _, body in pool.sent]
    assert len(set(payloads)) == 6
    assert {len(base64.b64decode(p)) for p in payloads} == {8}
    assert {json.loads(body)["namespace"] for _, _, body in pool.sent} == {7}


def test_drain_waits_for_the_query_node_to_show_the_idle_blocks():
    heights = heights_at(5, 7)

    def catch_up(now: float) -> None:
        if now >= 2.0:
            heights.saw("query", 8, now)

    clock = fakes.FakeClock(on_advance=catch_up)
    assert some(drain(netbench.LoadState(), heights, clock)) == pytest.approx(
        3.0, abs=0.2
    )


def test_drain_needs_a_counter_sample_after_the_idle_blocks(caplog):
    """Counters are polled every second: samples from before the last empty block say nothing
    about it. Here decided bytes grow in the first sample after the query node caught up."""
    heights = heights_at(5, 7)

    def catch_up(now: float) -> None:
        if now >= 2.5:
            heights.saw("query", 8, now)

    clock = fakes.FakeClock(on_advance=catch_up)
    elapsed = drain(
        netbench.LoadState(), heights, clock, decided=lambda ts: 1 + (ts >= 3)
    )
    assert some(elapsed) > 3.0 + netbench.DRAIN_IDLE_S - 1.0


def test_drain_target_resets_when_decided_bytes_grow():
    heights = heights_at(5, 5)

    def advance(now: float) -> None:
        if now >= 2.0:
            heights.saw("validator", 7, now)
        if now >= 9.0:
            heights.saw("query", 8, now)
        if now >= 12.0:
            heights.saw("query", 10, now)

    clock = fakes.FakeClock(on_advance=advance)
    elapsed = drain(
        netbench.LoadState(), heights, clock, decided=lambda ts: 1 + (ts >= 3)
    )
    assert some(elapsed) >= 12.0


def test_drain_gives_up_at_the_cap_while_blocks_keep_coming(caplog):
    heights = heights_at(5, 5)

    def blocks(now: float) -> None:
        heights.saw("validator", 5 + int(now // 5), now)

    clock = fakes.FakeClock(on_advance=blocks)
    assert drain(netbench.LoadState(), heights, clock, max_s=60.0) is None
    assert clock.now == pytest.approx(60.0, abs=0.5)
    assert "cap of 60 s" in caplog.text


def step_window() -> netbench.StepWindow:
    return {
        "rate_mb_s": 10.0,
        "refine": False,
        "kind": "ramp",
        "t_start": 0.0,
        "t_mid": 15.0,
        "t_end": 30.0,
    }


def measures(
    decided_mb_s: float = 10.0,
    timeouts: int = 0,
    consensus_s: float = 0.5,
    query_s: float = 0.2,
    query_growth: float = 0.0,
    now: float = math.inf,
) -> Any:
    """A 10 MB/s step, t 0 to 30, measured from 15: one 1 MB tx every 0.1 s."""
    txs = [{"t_submit": i / 10, "height": i, "status": "included"} for i in range(300)]
    heights = [
        {
            "height": i,
            "validator": i / 10 + consensus_s,
            "query": i / 10 + consensus_s + query_s + query_growth * i / 10,
            "scanned": None,
        }
        for i in range(300)
    ]
    counters = [
        {
            "ts": float(t),
            "decided_bytes": decided_mb_s * 1e6 * t,
            "timeouts": timeouts * (t > 20),
        }
        for t in range(31)
    ]
    return netbench.step_measures(
        step_window(), netbench.BenchConfig(), txs, heights, counters, now
    )


def fails(m: Any) -> tuple[list[str], list[str]]:
    return netbench.step_fails(m, netbench.BenchConfig())


def test_keeping_up():
    m = measures()
    assert m["decided_mb_s"] == pytest.approx(10.0)
    assert m["submitted_mb_s"] == pytest.approx(10.0, abs=0.1)
    assert m["timeouts"] == 0
    assert m["consensus_latency_ms"]["p50"] == pytest.approx(500.0)
    assert m["query_lag_ms"]["p50"] == pytest.approx(200.0)
    assert m["query_lag_slope_ms_s"] == pytest.approx(0.0)
    assert fails(m) == ([], [])


def test_consensus_rules():
    assert fails(measures(decided_mb_s=7.0, timeouts=1, consensus_s=1.5)) == (
        [
            "decided 70% of submitted",
            "1 view timeouts",
            "consensus latency p50 1500 ms > 1000 ms",
        ],
        [],
    )


def test_decided_is_judged_against_the_submitted_rate():
    """10 MB/s go out: 8.5 MB/s decided keeps up, whatever the step's nominal rate."""
    assert fails(measures(decided_mb_s=8.5)) == ([], [])
    assert fails(measures(decided_mb_s=7.9)) == (["decided 79% of submitted"], [])


def test_pending_transactions_count_once_over_target():
    """By t 20 all 20 are over the 1000 ms target; by t 18 the 10 submitted before 17."""
    cfg = netbench.BenchConfig()
    txs = [
        {"t_submit": 16.0 + i / 10, "height": None, "status": "pending"}
        for i in range(20)
    ]
    m = netbench.step_measures(step_window(), cfg, txs, [], [], 20.0)
    assert some(m["consensus_latency_ms"])["n"] == 20
    m = netbench.step_measures(step_window(), cfg, txs, [], [], 18.0)
    latency = some(m["consensus_latency_ms"])
    assert latency["n"] == 10
    assert latency["p50"] > 1000.0


NEW_STEP_KEYS = ("queued_mb_s", "queue_wait_ms", "submit_rtt_ms", "cap_waits")


def test_submit_measures_split_queue_wait_from_round_trip():
    """10 MB/s queued, each tx waiting 50 ms for a thread and 200 ms for the response."""
    txs = [
        {
            "t_queued": i / 10,
            "t_submit": i / 10 + 0.05,
            "t_done": i / 10 + 0.25,
            "height": None,
            "status": "pending",
        }
        for i in range(300)
    ]
    m = netbench.step_measures(step_window(), netbench.BenchConfig(), txs, [], [], 30.0)
    assert some(m["queued_mb_s"]) == pytest.approx(10.0, abs=0.1)
    assert some(m["queue_wait_ms"])["p50"] == pytest.approx(50.0)
    assert some(m["submit_rtt_ms"])["p50"] == pytest.approx(200.0)


def test_submit_measures_are_none_without_queue_timestamps():
    m = measures()
    assert (m["queued_mb_s"], m["queue_wait_ms"], m["submit_rtt_ms"]) == (
        None,
        None,
        None,
    )


@pytest.mark.parametrize(
    ("queued", "cap_waits", "wait_p50", "cause"),
    [
        (7.0, 3, 1.0, "in-flight cap reached (--cap-s)"),
        (7.0, 0, 1.0, "pacer late (controller CPU)"),
        (
            10.0,
            0,
            300.0,
            "submit workers busy (queue wait p50/p99 300/900 ms, 3 workers)",
        ),
        (10.0, 0, 1.0, "slow submit responses (rtt p50/p99 20/50 ms)"),
        (None, None, None, "cause not available"),
    ],
)
def test_summary_names_the_submission_shortfall(queued, cap_waits, wait_p50, cause):
    short = step(10.0) | {
        "submitted_mb_s": 9.0,
        "queued_mb_s": queued,
        "cap_waits": cap_waits,
        "queue_wait_ms": None if wait_p50 is None else quantiles(wait_p50, 900.0),
        "submit_rtt_ms": None if wait_p50 is None else quantiles(20.0, 50.0),
    }
    result = make_result([short])
    result["config"]["workers"] = 3
    summary = netbench.render(result, None)
    assert f"- submission short at 10 MB/s: {cause}" in summary


def test_unsent_and_unanswered_txs_count_with_their_time_so_far():
    """Queued at 20, still waiting at now 25; sent at 21, unanswered at now 25."""
    txs = [
        {"t_queued": 20.0, "t_submit": math.inf, "t_done": None},
        {"t_queued": 20.0, "t_submit": 21.0, "t_done": None},
    ]
    m = netbench.submit_measures(txs, 1_000_000, 15.0, 30.0, 25.0)
    assert some(m["queue_wait_ms"])["max"] == pytest.approx(5000.0)
    assert some(m["submit_rtt_ms"])["n"] == 1
    assert some(m["submit_rtt_ms"])["p50"] == pytest.approx(4000.0)


def test_old_steps_json_renders_without_queue_metrics(tmp_path):
    old = {k: v for k, v in step(10.0).items() if k not in NEW_STEP_KEYS}
    judged = old | {"consensus_fails": [], "query_fails": []}
    result = netbench.step_result(judged, [], {}, [])
    assert [result[k] for k in NEW_STEP_KEYS] == [None] * len(NEW_STEP_KEYS)
    assert "10" in netbench.render(make_result([result]), None)


def test_summary_has_no_shortfall_line_when_submission_keeps_up():
    assert "submission short" not in netbench.render(make_result(), None)


def test_decided_rate_is_not_quantized_by_blocks():
    """A 20 MB block every 2 s at t 1, 3, 5, ...: 10 MB/s, but the counter samples at 15 and
    30 see 7 blocks in 15 s."""
    counters = [
        {"ts": float(t), "decided_bytes": 20e6 * ((t + 1) // 2), "timeouts": 0}
        for t in range(31)
    ]
    m = netbench.step_measures(
        step_window(), netbench.BenchConfig(), [], [], counters, 30.0
    )
    assert some(m["decided_mb_s"]) == pytest.approx(10.0, abs=0.3)
    assert fails(m) == ([], [])


def test_growing_query_lag():
    m = measures(query_growth=0.2)
    assert m["query_lag_slope_ms_s"] == pytest.approx(200.0, abs=1)
    consensus, query = fails(m)
    assert consensus == []
    assert query[0] == "query lag grows 200 ms/s"


def test_heights_not_yet_on_the_query_node_count_as_lagging():
    """At t 22 nothing is on the query node yet: heights older than the target lag."""
    m = measures(query_s=100.0, now=22.0)
    assert m["query_lag_ms"]["p50"] > 1000.0
    assert "query lag p50" in fails(m)[1][0]


def verdict(rate, consensus=(), query=()):
    return {
        "kind": "ramp",
        "rate_mb_s": rate,
        "submitted_mb_s": rate,
        "decided_mb_s": rate,
        "consensus_fails": [*consensus],
        "query_fails": [*query],
    }


DECIDED_90 = ["decided 90% of offered"]
QUERY_LAG = ["query lag p50 1500 ms > 1000 ms"]


@pytest.mark.parametrize(
    ("verdicts", "overall", "line"),
    [
        (
            [verdict(4.0), verdict(6.0)],
            {"mb_s": 6.0, "bounded": False, "failed_at_mb_s": None},
            "Capacity **>= 6 MB/s**: no step failed.",
        ),
        (
            [verdict(4.0), verdict(6.0), verdict(8.0, DECIDED_90), verdict(7.0)],
            {"mb_s": 7.0, "bounded": True, "failed_at_mb_s": 8.0},
            "Capacity **7 MB/s** (+-1): consensus limits (decided 90% of offered at 8 MB/s).",
        ),
        (
            [verdict(4.0), verdict(6.0, query=QUERY_LAG), verdict(5.0)],
            {"mb_s": 5.0, "bounded": True, "failed_at_mb_s": 6.0},
            (
                "Capacity **5 MB/s** (+-1): query node limits at 5 MB/s, consensus >= 6 MB/s "
                "(query lag p50 1500 ms > 1000 ms at 6 MB/s)."
            ),
        ),
        (
            [verdict(4.0, ["1 view timeouts"])],
            {"mb_s": None, "bounded": True, "failed_at_mb_s": 4.0},
            "Capacity **< 4 MB/s**: consensus limits (1 view timeouts at 4 MB/s).",
        ),
    ],
)
def test_capacity(verdicts, overall, line):
    cap = netbench.capacity(verdicts)
    assert cap["overall"] == overall
    assert netbench.capacity_line(cap) == line


@pytest.mark.parametrize(
    ("buckets", "total_s", "expected"),
    [
        (
            {"0.1": 50.0, "0.2": 100.0, "+Inf": 100.0},
            10.0,
            {"p50": 100.0, "p99": 198.0, "mean": 100.0},
        ),
        (
            {"0.1": 50.0, "+Inf": 100.0},
            30.0,
            {"p50": 100.0, "p99": 100.0, "max": 100.0},
        ),
    ],
)
def test_histogram_quantiles(buckets, total_s, expected):
    m1 = {f'op_bucket{{le="{le}"}}': n for le, n in buckets.items()} | {
        "op_count": 100.0,
        "op_sum": total_s,
    }
    q = some(netbench.histogram_quantiles({}, m1, "op"))
    assert {k: q[k] for k in expected} == pytest.approx(expected)


def test_stale_connection_is_retried_on_a_fresh_one(monkeypatch: pytest.MonkeyPatch):
    connections = fakes.FakeConnections(b"42")
    monkeypatch.setattr(netbench.http.client, "HTTPConnection", connections)
    pool = netbench.HttpPool(fakes.FakeClock())
    assert pool.request("GET", "http://x/y") == (200, b"42")
    assert pool.request("GET", "http://x/y") == (200, b"42")
    stale, fresh = connections.made
    assert (stale.closed, stale.requests, fresh.requests) == (True, 2, 1)


def test_window_rate_counts_transactions_in_the_window():
    times = [50.0, 80.0, 99.0, math.inf]
    assert netbench.window_mb_s(times, 1_500_000, 70.0, 100.0) == pytest.approx(0.1)


def progress_line(
    caplog: pytest.LogCaptureFixture,
    phase: netbench.Phase,
    txs: int,
    height: int,
    now: float = 100.0,
    block_mb: float | None = None,
) -> str:
    """`txs` 1 MB txs submitted 6 s before `now`, a third of them in a block at `height` that
    validators showed 5 s and the query node 4.5 s before `now`, scanned at `now`. The block
    holds `block_mb`, default half the txs."""
    state = netbench.LoadState()
    state.phase = phase
    for i in range(txs):
        state.submitted(netbench.Tx(id=i, node=0, t_queued=0.0, t_submit=now - 6))
    for i in range(txs // 3):
        state.include(i, height=height, at=now - 4.5)
    heights = netbench.Heights(height)
    heights.saw("validator", height + 1, now - 5)
    heights.saw("query", height + 1, now - 4.5)
    heights.scanned[height] = now
    counters = [
        {"ts": now - 10, "decided_bytes": 0, "timeouts": 0},
        {"ts": now, "decided_bytes": (block_mb or txs / 2) * 1e6, "timeouts": 1},
    ]
    caplog.clear()
    with caplog.at_level(logging.INFO, netbench.log.name):
        netbench.log_progress(state, heights, counters, now, 1_000_000, 2)
    (line,) = caplog.messages
    return line


def test_progress_line(caplog: pytest.LogCaptureFixture):
    line = progress_line(
        caplog, netbench.Phase("probe 2 climb", 60.0, 80.0, 60.0), 90, 0
    )
    assert line == (
        "probe 2 climb      60.0 MB/s     20/60s"
        " | sub   9.0 MB/s pend     60 to     2"
        " | cns   4.5 MB/s  50% h       0 blk 10.00s  45.0MB lat   1000ms"
        " | qry   3.0 MB/s h       0 lag    500ms | vto 1"
    )


def test_progress_columns_align_across_phases_and_magnitudes(
    caplog: pytest.LogCaptureFixture,
):
    lines = [
        progress_line(caplog, netbench.Phase("probe 2 climb", 0.5, 80.0, 60.0), 9, 0),
        progress_line(caplog, netbench.Phase("drain", None, 1.0, 300.0), 0, 5),
        progress_line(
            caplog,
            netbench.Phase("probe 12 recovery", 9999.0, 0.0, 1800.0),
            99_990,
            9_999_998,
            1800.0,
            999.9,
        ),
        progress_line(caplog, netbench.Phase("starting", None, 0.0, 0.0), 0, 0),
    ]
    columns = {tuple(i for i, c in enumerate(line) if c == "|") for line in lines}
    assert len(columns) == 1, "\n".join(lines)


SEARCH = netbench.SearchConfig(
    start_mb_s=100.0, resolution_mb_s=10.0, max_probes=12, offered_gb=150.0
)
SEARCH_CFG = netbench.BenchConfig(step_s=60, warmup_s=0)
SOFT = ["consensus latency p50 1400 ms > 1000 ms"]


def probe_step(
    rate: float,
    kind: str = "climb",
    decided: float = 1.0,
    consensus: list[str] | None = None,
    query: list[str] | None = None,
    cap_waits: int = 0,
    duration: float = 60.0,
) -> dict[str, Any]:
    """`decided` is the fraction of the rate; no rule fails unless given."""
    return {
        "rate_mb_s": rate,
        "kind": kind,
        "refine": False,
        "submitted_mb_s": rate,
        "decided_mb_s": rate * decided,
        "consensus_fails": consensus or [],
        "query_fails": query or [],
        "cap_waits": cap_waits,
        "t_start": 0.0,
        "t_end": duration,
    }


def passes(*rates: float, kind: str = "climb") -> list[dict[str, Any]]:
    return [probe_step(r, kind) for r in rates]


def soft_fail(rate: float, kind: str = "bisect") -> dict[str, Any]:
    return probe_step(rate, kind, 0.9, SOFT)


def collapse(rate: float, kind: str = "climb") -> dict[str, Any]:
    return probe_step(rate, kind, 0.4, ["decided 40% of submitted"])


def probe(steps, search=SEARCH, cfg=SEARCH_CFG, side="overall"):
    return netbench.next_probe(steps, search, cfg, side)


def as_tuple(p) -> Any:
    return p if isinstance(p, str) else (p["rate_mb_s"], p["kind"], p["duration_s"])


@pytest.mark.parametrize(
    ("steps", "expected"),
    [
        ([], (100.0, "climb", 60)),
        (passes(100.0), (125.0, "climb", 60)),
        (passes(100.0, 125.0), (156.25, "climb", 60)),
    ],
)
def test_next_probe_climbs(steps, expected):
    assert as_tuple(probe(steps)) == expected


def test_next_probe_bisects_until_the_bracket_is_within_resolution():
    steps = [*passes(100.0, 125.0), soft_fail(156.25, "climb")]
    assert as_tuple(probe(steps)) == (140.625, "bisect", 60)
    steps.append(probe_step(140.625, "bisect"))
    assert as_tuple(probe(steps)) == (148.4375, "bisect", 60)
    steps.append(probe_step(148.4375, "bisect"))
    assert as_tuple(probe(steps)) == (148.4375, "confirm", 120)


def test_next_probe_recovers_after_a_collapse():
    steps = [*passes(100.0, 125.0), collapse(156.25)]
    assert as_tuple(probe(steps)) == (125.0, "recovery", 60)
    ok = [*steps, probe_step(125.0, "recovery")]
    assert as_tuple(probe(ok)) == (140.625, "bisect", 60)
    bad = [*steps, probe_step(125.0, "recovery", 0.9, SOFT)]
    assert as_tuple(probe(bad)) == (112.5, "bisect", 60)


def test_a_collapsed_recovery_steps_down_to_the_next_lower_pass():
    """Run lulu-20261008-092203: 293 passed at its edge, its recovery collapsed; 234 below it
    was never retried."""
    steps = [*passes(150.0, 188.0, 234.0, 293.0), collapse(366.0)]
    steps.append(collapse(293.0, "recovery"))
    assert as_tuple(probe(steps)) == (234.0, "recovery", 60)
    steps.append(probe_step(234.0, "recovery"))
    assert as_tuple(probe(steps)) == (263.5, "bisect", 60)


def test_a_collapsed_first_probe_is_retried_at_start():
    steps = [collapse(100.0)]
    assert as_tuple(probe(steps)) == (100.0, "recovery", 60)
    assert probe([*steps, probe_step(100.0, "recovery", 0.9, SOFT)]) == "below start"


def test_a_passing_retry_of_the_first_probe_forgives_its_collapse():
    steps = [collapse(100.0), probe_step(100.0, "recovery")]
    assert netbench.bracket(steps, "overall") == (100.0, None)
    assert as_tuple(probe(steps)) == (125.0, "climb", 60)
    assert netbench.capacity(steps)["overall"]["bounded"] is False


def test_next_probe_confirms_the_resolved_bracket_and_resumes_when_it_fails():
    steps = [
        *passes(100.0, 125.0),
        soft_fail(156.25, "climb"),
        probe_step(140.625, "bisect"),
        soft_fail(148.4375),
    ]
    assert as_tuple(probe(steps)) == (140.625, "confirm", 120)
    steps.append(soft_fail(140.625, "confirm"))
    assert as_tuple(probe(steps)) == (132.8125, "bisect", 60)
    wide = dataclasses.replace(SEARCH, resolution_mb_s=20.0)
    assert as_tuple(probe(steps, wide)) == (125.0, "confirm", 120)


def test_next_probe_stops_resolved_after_a_passing_confirm():
    steps = [
        *passes(100.0, 125.0),
        soft_fail(156.25, "climb"),
        probe_step(148.4375, "bisect"),
        probe_step(148.4375, "confirm"),
    ]
    assert probe(steps) == "resolved"


def test_next_probe_stops_at_the_budgets():
    assert probe(passes(*range(10, 22))) == "probe budget"
    small = dataclasses.replace(SEARCH, offered_gb=120.0)
    big = [probe_step(100.0, duration=1150.0)]
    assert netbench.offered_gb(big, SEARCH_CFG) == pytest.approx(115.0)
    assert probe(big, small) == "disk budget"
    assert probe([], dataclasses.replace(SEARCH, offered_gb=1.0)) == "disk budget"


def query_bound_steps() -> list[dict[str, Any]]:
    return [
        *passes(100.0, 200.0),
        probe_step(210.0, "bisect", query=["query lag grows 76 ms/s"]),
        probe_step(220.0, "bisect", query=["query lag grows 59 ms/s"]),
    ]


def test_a_query_bound_search_continues_on_the_consensus_side():
    steps = query_bound_steps()
    assert netbench.search_sides(steps) == "consensus"
    assert netbench.search_sides([*steps, collapse(300.0)]) is None
    assert netbench.search_sides(passes(100.0)) is None
    assert as_tuple(probe(steps, side="consensus")) == (275.0, "climb", 60)
    throttled = [*steps[:-1], {**steps[-1], "cap_waits": 3}]
    assert probe(throttled, side="consensus") == "generator throttled"


def test_the_first_consensus_probe_above_the_query_limit_can_fail():
    steps = [*query_bound_steps(), collapse(275.0)]
    cap = netbench.capacity(steps)
    assert cap["consensus"] == {"mb_s": 220.0, "bounded": True, "failed_at_mb_s": 275.0}


@pytest.mark.parametrize(
    ("steps", "expected"),
    [
        ([soft_fail(100.0, "climb")], "below start"),
        ([collapse(100.0), collapse(100.0, "recovery")], "below start"),
        (
            [*passes(100.0), collapse(125.0), collapse(100.0, "recovery")],
            "degraded after overload at 125",
        ),
    ],
)
def test_next_probe_stops_below_start_or_degraded(steps, expected):
    assert probe(steps) == expected


def test_a_collapsed_confirm_is_followed_by_a_recovery_at_the_next_lower_pass():
    steps = [
        *passes(100.0, 125.0, 150.0),
        soft_fail(160.0),
        collapse(150.0, "confirm"),
    ]
    assert as_tuple(probe(steps)) == (125.0, "recovery", 60)


def test_the_bracket_ignores_passes_above_the_lowest_failure():
    steps = [*passes(200.0), soft_fail(210.0), *passes(205.0, 215.0)]
    assert netbench.bracket(steps, "overall") == (205.0, 210.0)


def test_a_bracket_at_resolution_is_confirmed_without_repeating_a_rate():
    tiny = dataclasses.replace(SEARCH, resolution_mb_s=0.02)
    steps = [*passes(0.06), soft_fail(0.08)]
    assert as_tuple(probe(steps, tiny)) == (0.06, "confirm", 120)


def test_search_log_prefix_shows_the_bracket():
    steps = [*passes(187.5), soft_fail(210.9)]
    p: netbench.Probe = {"rate_mb_s": 199.2, "kind": "bisect", "duration_s": 60}
    assert (
        netbench.search_log_prefix(steps, p, "overall")
        == "probe 3 bisect 199 MB/s [lo 188 hi 211]"
    )


def test_search_staircase_converges_within_resolution(staircase, tmp_path: Path):
    search = netbench.SearchConfig(0.05, 0.01, 12, 1.0)
    run = staircase(steps=(0.05,), tx_timeout_s=1, search=search)
    kinds = [s["kind"] for s in run.steps]
    assert kinds[0] == "climb"
    assert kinds[-1] == "confirm"
    assert kinds == sorted(kinds, key=["climb", "bisect", "confirm"].index)
    assert len(kinds) <= 8
    cap = netbench.capacity(run.steps)
    assert 0.07 <= some(cap["overall"]["mb_s"]) <= 0.08
    assert cap["confirmed"]
    assert some(cap["resolution_mb_s"]) <= 0.01 + 1e-9
    assert run.meta["stop_reason"] == "resolved"


def test_search_writes_steps_json_after_every_probe(tmp_path: Path):
    seen: list[int] = []

    def watch(now: float) -> None:
        path = tmp_path / "steps.json"
        if path.exists() and (n := len(netbench.read_json(path))) not in seen:
            seen.append(n)

    clock = fakes.FakeClock(on_advance=watch)
    node = fakes.FakeNode(clock, include=True, block_txs=1)
    cfg = netbench.BenchConfig(
        tx_size=4000,
        workers=3,
        steps=(0.05,),
        step_s=int(STEP_S),
        warmup_s=0,
        tx_timeout_s=1,
    )
    search = netbench.SearchConfig(0.05, 0.01, 12, 1.0)
    clock.run(
        netbench.generate_load(
            cfg,
            [node.url],
            [node.url],
            [node.url] * 2,
            tmp_path,
            clock,
            node.connect,
            search=search,
        )
    )
    assert seen[:2] == [1, 2]


def test_search_drains_only_after_failing_probes(staircase, monkeypatch):
    drained: list[int] = []
    real = netbench.drain

    async def spy(*args: Any, **kwargs: Any) -> float | None:
        drained.append(len(args[0].txs))
        return await real(*args, **kwargs)

    monkeypatch.setattr(netbench, "drain", spy)
    search = netbench.SearchConfig(0.05, 0.01, 12, 1.0)
    run = staircase(steps=(0.05,), tx_timeout_s=1, search=search)
    failing = sum(bool(s["consensus_fails"] + s["query_fails"]) for s in run.steps[:-1])
    assert len(drained) == failing


def test_search_stops_when_the_backlog_does_not_drain(staircase, monkeypatch):
    async def undrained(*_: Any, **__: Any) -> None:
        return None

    monkeypatch.setattr(netbench, "drain", undrained)
    search = netbench.SearchConfig(0.05, 0.01, 12, 1.0)
    run = staircase(steps=(0.05,), tx_timeout_s=1, search=search)
    assert run.meta["stop_reason"] == "drain timeout"
    assert netbench.capacity(run.steps)["overall"]["bounded"]


def test_one_passing_run_does_not_hide_a_failing_probe_at_the_same_rate():
    cap = netbench.capacity(
        [*passes(100.0, 150.0), soft_fail(150.0, "confirm"), *passes(125.0)]
    )
    assert cap["overall"] == {"mb_s": 125.0, "bounded": True, "failed_at_mb_s": 150.0}
    assert cap["resolution_mb_s"] == 25.0
    assert not cap["confirmed"]


def test_confirmed_needs_a_passing_confirm_at_the_limit():
    steps = [*passes(100.0), probe_step(100.0, "confirm"), soft_fail(110.0)]
    assert netbench.capacity(steps)["confirmed"]
    assert not netbench.capacity([*passes(100.0), soft_fail(110.0)])["confirmed"]


def test_query_limit_ignores_steps_whose_consensus_failed():
    steps = [
        *passes(100.0),
        probe_step(150.0, "bisect", 0.4, ["decided 40% of submitted"]),
    ]
    assert netbench.capacity(steps)["query_node"] == {
        "mb_s": 100.0,
        "bounded": False,
        "failed_at_mb_s": None,
    }


def test_a_collapsed_step_that_decided_nothing_does_not_invalidate_the_run():
    steps = [step(4.0), step(6.0, 0.0, FAIL)]
    validity = netbench.check_validity(make_result(steps), ALL_ANSWERED)
    assert validity["valid"]


def test_stop_reason_reaches_the_summary(tmp_path: Path):
    write_run_dir(tmp_path)
    meta = netbench.read_json(tmp_path / "load-meta.json")
    netbench.write_json(tmp_path / "load-meta.json", meta | {"stop_reason": "resolved"})
    result = analyze(tmp_path)
    assert result["load"]["stop_reason"] == "resolved"
    assert "search: resolved" in netbench.render(result, None)


def test_a_load_without_stop_reason_reads_none(tmp_path: Path):
    write_run_dir(tmp_path)
    result = analyze(tmp_path)
    assert result["load"]["stop_reason"] is None
    assert "search:" not in netbench.render(result, None)


def test_old_steps_json_without_kind_renders_ramp_and_refine(tmp_path: Path):
    write_run_dir(tmp_path)
    steps = netbench.read_json(tmp_path / "steps.json")
    old = [{k: v for k, v in s.items() if k != "kind"} for s in steps]
    old[1]["refine"] = True
    netbench.write_json(tmp_path / "steps.json", old)
    result = analyze(tmp_path)
    assert [s["kind"] for s in result["steps"]] == ["ramp", "refine"]
    assert "(refine)" in netbench.render(result, None)


def test_compare_uses_the_stored_resolution():
    current = make_result([step(150.0)])
    current["config"]["steps"] = [150.0]
    current["capacity"]["resolution_mb_s"] = 5.8
    comparison = compare(current, [make_result([step(150.0)])])
    assert row(comparison, "capacity")["resolution_mb_s"] == 5.8
    current["capacity"]["resolution_mb_s"] = None
    comparison = compare(current, [make_result([step(150.0)])])
    assert row(comparison, "capacity")["resolution_mb_s"] == 75.0


def test_legacy_config_hash_keeps_its_value():
    assert netbench.config_hash(netbench.BenchConfig(), []) == "bdf88113f497"


def test_heartbeat_changes_the_config_hash():
    beating = netbench.BenchConfig(heartbeat_tx_s=50.0)
    assert netbench.config_hash(beating, []) != "bdf88113f497"


def test_legacy_staircase_reports_why_it_stopped(staircase):
    assert staircase(steps=(0.02, 0.04, 0.16), tx_timeout_s=1).meta["stop_reason"] == (
        "refined"
    )
    assert staircase(steps=(0.02,), tx_timeout_s=1).meta["stop_reason"] == (
        "ramp exhausted"
    )
    assert staircase(steps=(0.16,), tx_timeout_s=1).meta["stop_reason"] == (
        "first step failed"
    )


def test_a_confirm_that_fails_consensus_bounds_the_consensus_limit_below_it():
    query = ["query lag grows 76 ms/s"]
    steps = [
        *passes(200.0),
        probe_step(244.1, "climb", query=query),
        probe_step(274.6, "bisect", consensus=SOFT),
        probe_step(259.4, "bisect", query=query),
        probe_step(259.4, "confirm", consensus=SOFT),
    ]
    assert netbench.capacity(steps)["consensus"] == {
        "mb_s": 244.1,
        "bounded": True,
        "failed_at_mb_s": 259.4,
    }


def test_a_collapsed_confirm_at_the_start_rate_ends_the_search():
    steps = [
        *passes(100.0),
        soft_fail(125.0, "climb"),
        soft_fail(112.5),
        soft_fail(106.25),
        collapse(100.0, "confirm"),
        probe_step(100.0, "recovery"),
    ]
    assert probe(steps) == "below start"


def test_a_forgiven_first_probe_does_not_hide_a_query_bound_search():
    steps = [
        collapse(100.0),
        probe_step(100.0, "recovery"),
        probe_step(125.0, "climb", query=["query lag grows 76 ms/s"]),
    ]
    assert netbench.search_sides(steps) == "consensus"


def test_a_keep_going_ramp_reports_that_it_ran_every_step(staircase):
    run = staircase(steps=(0.02, 0.16, 0.04), tx_timeout_s=1, keep_going=True)
    assert run.meta["stop_reason"] == "ramp exhausted"


def test_step_table_renders_steps_from_before_kind():
    current = make_result([step(4.0), step(6.0)])
    for s in current["steps"]:
        cast(dict[str, Any], s).pop("kind")
    current["steps"][1]["refine"] = True
    table = "\n".join(netbench.step_table(current, None))
    assert "| 6 (refine) |" in table


def test_query_nodes_come_from_the_roles():
    assert netbench.query_nodes(TOPOLOGY) == ["node0"]
    roles = {"a": "validator, query, sqlite", "b": "validator", "c": "validator, query"}
    topo: netbench.Topology = {
        "nodes": dict.fromkeys(roles, "http://x"),
        "roles": roles,
        "query_node": "a",
    }
    assert netbench.query_nodes(topo) == ["a", "c"]


@dataclasses.dataclass
class ClusterLoad:
    cluster: fakes.FakeCluster
    load: Load


def run_cluster_load(
    out: Path,
    duration: float,
    names: list[str],
    query: set[str],
    on_advance: Callable[[fakes.FakeCluster, float], None] | None = None,
    **cluster_kwargs: Any,
) -> ClusterLoad:
    """5 txs/s of 1000 bytes for `duration` clock seconds, submitted to every node of a
    `FakeCluster`; the clock waits on threads, so a downed node's readers can retry."""
    holder: list[fakes.FakeCluster] = []

    def advance(now: float) -> None:
        if on_advance is not None and holder:
            on_advance(holder[0], now)

    clock = fakes.FakeClock(threaded=True, on_advance=advance)
    cluster = fakes.FakeCluster(clock, names, query, **cluster_kwargs)
    holder.append(cluster)
    topo = cluster.topology()
    queries = netbench.query_nodes(topo)
    cfg = netbench.BenchConfig(
        tx_size=1000,
        workers=3,
        steps=(0.005,),
        step_s=int(duration),
        warmup_s=0,
        cap_s=200.0,
        tx_timeout_s=5,
        submit_nodes=len(names),
    )
    urls = [topo["nodes"][n] for n in names]
    load = netbench.generate_load(
        cfg,
        urls,
        [topo["nodes"][n] for n in queries],
        [url for n, url in topo["nodes"].items() if n not in queries],
        out,
        clock,
        cluster.connect,
    )
    clock.run(load)
    return ClusterLoad(
        cluster,
        Load(
            fakes.FakeNode(clock, False),
            list(netbench.read_jsonl(out / "load.jsonl")),
            netbench.read_json(out / "load-meta.json"),
            netbench.read_json(out / "steps.json"),
            list(netbench.read_jsonl(out / "heights.jsonl")),
        ),
    )


def test_a_submit_to_a_stopped_node_goes_to_the_next(tmp_path: Path):
    def stop_node1(cluster: fakes.FakeCluster, now: float) -> None:
        if cluster.members["node1"].running and now > 0:
            cluster.kill("node1")

    run = run_cluster_load(
        tmp_path, 4.0, ["node0", "node1", "node2"], {"node0"}, stop_node1
    )
    assert run.load.txs
    assert included(run.load.txs)
    assert run.load.meta["submit_failovers"] > 0
    assert run.load.meta["submit_errors"] == 0
    assert run.cluster.members["node1"].submits == []
    assert 1 not in {tx["node"] for tx in run.load.txs[3:]}


def cluster_load_parts(
    names: list[str], query: set[str]
) -> tuple[fakes.FakeClock, fakes.FakeCluster, netbench.Load]:
    clock = fakes.FakeClock(threaded=True)
    cluster = fakes.FakeCluster(clock, names, query)
    cfg = netbench.BenchConfig(tx_size=1000, workers=1, submit_nodes=len(names))
    marker = b"m" * 16
    load = netbench.Load(
        cfg,
        netbench.LoadState(),
        netbench.Client(cluster.connect(clock), ThreadPoolExecutor(1)),
        list(cluster.urls.values()),
        netbench.TxBodies(cfg, marker),
        clock,
    )
    return clock, cluster, load


def test_a_tx_no_node_takes_is_one_submit_error_and_dropped():
    clock, cluster, load = cluster_load_parts(["node0", "node1"], {"node0"})
    for name in cluster.members:
        cluster.kill(name)
    tx = netbench.Tx(id=0, node=1, t_queued=0.0)
    load.state.submitted(tx)
    clock.run(netbench.submit_tx(load, tx))
    assert load.state.submit_errors == 1
    assert load.state.txs == []
    assert load.state.pending == {}


def test_a_failed_over_tx_stays_pending_and_records_where_it_landed():
    clock, cluster, load = cluster_load_parts(["node0", "node1", "node2"], {"node0"})
    cluster.kill("node1")
    tx = netbench.Tx(id=1, node=1, t_queued=0.0)
    load.state.submitted(tx)
    clock.run(netbench.submit_tx(load, tx))
    assert (tx.node, load.state.failovers, load.state.submit_errors) == (2, 1, 0)
    assert list(load.state.pending) == [1]
    assert len(cluster.members["node2"].submits) == 1


def test_a_post_tries_each_url_once():
    clock, cluster, _ = cluster_load_parts(["node0", "node1", "node2"], {"node0"})
    for name in cluster.members:
        cluster.kill(name)
    pool = cluster.connect(clock)
    urls = list(cluster.urls.values())
    assert netbench.post_failover(pool, urls, 1, b"{}")[:2] == (0, 0)


def test_the_heartbeat_fails_over_to_the_next_url():
    stop = threading.Event()
    pool = BeatPool(4, stop, down=("http://a",))
    failed = netbench.heartbeat(
        cast("Any", pool), ["http://a", "http://b"], 7, 50.0, stop
    )
    assert failed == 0
    assert {url for _, url, _ in pool.sent[1:]} == {
        "http://a/v1/submit/submit",
        "http://b/v1/submit/submit",
    }


def test_a_heartbeat_no_url_takes_counts_as_failed():
    stop = threading.Event()
    pool = BeatPool(4, stop, down=("http://",))
    failed = netbench.heartbeat(
        cast("Any", pool), ["http://a", "http://b"], 7, 50.0, stop
    )
    assert failed == 2


def test_a_payload_comes_from_the_next_query_node_on_404_or_refusal():
    clock = fakes.FakeClock()
    cluster = fakes.FakeCluster(clock, ["node0", "node1", "node2"], {"node0", "node1"})
    clock.advance(2.0)
    pool = cluster.connect(clock)
    urls = [cluster.urls["node0"], cluster.urls["node1"]]
    cluster.wipe("node0")
    cluster.start("node0")
    assert netbench.block_payload(pool, urls, 10) == b""
    cluster.kill("node0")
    assert netbench.block_payload(pool, urls, 10) == b""
    assert netbench.block_payload(pool, urls, 10_000) is None
    cluster.kill("node1")
    assert netbench.block_payload(pool, urls, 10) is None


def test_scan_block_finds_ids_on_the_second_query_node():
    clock = fakes.FakeClock()
    cluster = fakes.FakeCluster(clock, ["node0", "node1", "node2"], {"node0", "node1"})
    marker = b"m" * 16
    body = json.dumps(
        {
            "namespace": 1,
            "payload": base64.b64encode(marker + (7).to_bytes(8, "big")).decode(),
        }
    ).encode()
    pool = cluster.connect(clock)
    pool.request("POST", cluster.urls["node2"] + "/v1/submit/submit", body)
    clock.advance(0.2)
    cluster.wipe("node0")
    cluster.start("node0")
    urls = [cluster.urls["node0"], cluster.urls["node1"]]
    found = [netbench.scan_block(pool, urls, marker, h) for h in range(1, 4)]
    assert [7] in found
    assert None not in found


def test_a_height_poller_survives_an_outage_and_resumes():
    def outage(now: float) -> None:
        if now >= 5.0 and cluster.members["node1"].running and now < 45.0:
            cluster.kill("node1")
        elif now >= 45.0 and not cluster.members["node1"].running:
            cluster.start("node1")

    clock = fakes.FakeClock(threaded=True, on_advance=outage)
    cluster = fakes.FakeCluster(clock, ["node0", "node1"], {"node0"})
    heights = netbench.Heights(0)

    async def main() -> None:
        client = netbench.Client(cluster.connect(clock), ThreadPoolExecutor(2))
        task = asyncio.create_task(
            netbench.poll_heights(
                client, cluster.urls["node1"], heights, "validator", clock
            )
        )
        await clock.asleep(80.0)
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task

    clock.run(main())
    seen = sorted(heights.seen["validator"].values())
    assert seen[0] < 5.0
    assert seen[-1] > 70.0
    assert not [t for t in seen if 6.0 < t < 44.0]


def run_counters(
    on_advance: Callable[[fakes.FakeCluster, float], None], seconds: float
) -> list[dict[str, Any]]:
    holder: list[fakes.FakeCluster] = []
    clock = fakes.FakeClock(on_advance=lambda now: on_advance(holder[0], now))
    cluster = fakes.FakeCluster(clock, ["node0", "node1", "node2", "node3"], {"node3"})
    holder.append(cluster)
    counters: list[dict[str, Any]] = []
    urls = [cluster.urls[n] for n in ("node0", "node1", "node2")]
    tx = json.dumps({"namespace": 1, "payload": base64.b64encode(b"x" * 100).decode()})

    async def main() -> None:
        client = netbench.Client(cluster.connect(clock), ThreadPoolExecutor(3))
        task = asyncio.create_task(
            netbench.poll_counters(client, urls, counters, clock)
        )
        for _ in range(int(seconds)):
            cluster.request(
                "POST", cluster.urls["node3"] + "/v1/submit/submit", tx.encode()
            )
            await clock.asleep(1.0)
        task.cancel()

    clock.run(main())
    return counters


def test_decided_bytes_never_decrease_across_a_validator_restart():
    def restart(cluster: fakes.FakeCluster, now: float) -> None:
        for name, at in (("node0", 10.0), ("node1", 15.0)):
            member = cluster.members[name]
            if now >= at and member.started_at < at:
                cluster.restart(name)

    counters = run_counters(restart, 25)
    decided = [c["decided_bytes"] for c in counters]
    assert len(decided) >= 20
    assert decided == sorted(decided)
    assert decided[-1] > 0


def test_a_counters_sweep_nobody_answers_writes_no_sample():
    def outage(cluster: fakes.FakeCluster, now: float) -> None:
        for name in ("node0", "node1", "node2"):
            member = cluster.members[name]
            if 5.0 <= now < 10.0 and member.running:
                cluster.kill(name)
            elif now >= 10.0 and not member.running:
                cluster.start(name)

    counters = run_counters(outage, 15)
    stamps = [c["ts"] for c in counters]
    assert not [t for t in stamps if 5.5 < t < 9.5]
    assert stamps[0] < 5.0
    assert stamps[-1] > 10.0


def test_query_heights_keep_advancing_with_one_query_node_down(tmp_path: Path):
    def stop_node0(cluster: fakes.FakeCluster, now: float) -> None:
        if cluster.members["node0"].running and now > 0:
            cluster.kill("node0")

    run = run_cluster_load(
        tmp_path, 40.0, ["node0", "node1", "node2"], {"node0", "node1"}, stop_node0
    )
    seen = [h["query"] for h in run.load.heights if h["query"] is not None]
    assert len(seen) > 5
    assert max(seen) > 30.0


def test_the_window_takes_the_highest_query_node():
    key = "consensus_last_synced_block_height"
    series: dict[str, list[netbench.Sample]] = {
        "node0": [(0.0, {key: 100.0}), (10.0, {key: 10.0})],
        "node1": [(0.0, {key: 90.0}), (10.0, {key: 200.0})],
        "node2": [(0.0, {key: 500.0}), (10.0, {key: 900.0})],
    }
    assert netbench.window(series, ["node0", "node1"], 0.0, 10.0) == {
        "t0": 0.0,
        "t1": 10.0,
        "height_start": 100,
        "height_end": 200,
    }


def test_drive_load_splits_validators_from_query_nodes(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    clock = fakes.FakeClock()
    cluster = fakes.FakeCluster(
        clock, ["node0", "node1", "node2", "node3"], {"node0", "node1"}
    )
    seen: list[Any] = []

    async def spy(*args: Any) -> tuple[float, float]:
        seen.extend(args[1:4])
        return 0.0, 1.0

    monkeypatch.setattr(netbench, "generate_load", spy)
    netbench.drive_load(
        netbench.BenchConfig(), cluster.topology(), tmp_path, lambda: True, clock,
        cluster.connect,
    )  # fmt: skip
    urls = cluster.urls
    assert seen == [
        [urls["node2"], urls["node3"], urls["node0"], urls["node1"]],
        [urls["node0"], urls["node1"]],
        [urls["node2"], urls["node3"]],
    ]


def test_a_node_down_at_the_end_has_no_final_metrics(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    clock = fakes.FakeClock()
    cluster = fakes.FakeCluster(clock, ["node0", "node1", "node2"], {"node0"})
    cluster.kill("node2")

    async def done(*_: Any) -> tuple[float, float]:
        return 0.0, 1.0

    monkeypatch.setattr(netbench, "generate_load", done)
    netbench.drive_load(
        netbench.BenchConfig(), cluster.topology(), tmp_path, lambda: True, clock,
        cluster.connect,
    )  # fmt: skip
    assert sorted(p.name for p in tmp_path.glob("final-*")) == [
        "final-node0.prom",
        "final-node1.prom",
    ]


def test_a_tx_included_twice_after_a_failover_resolves_once():
    state = netbench.LoadState()
    tx = netbench.Tx(id=3, node=0, t_queued=0.0, t_submit=0.0)
    state.submitted(tx)
    state.include(3, 10, 1.0)
    state.include(3, 12, 2.0)
    assert (tx.height, tx.t_included, tx.status) == (10, 1.0, "included")
    assert state.timeouts == 0


def test_faulted_nodes_are_exempt_from_coverage_and_decided_blocks():
    result = edited({"nodes.node2.decided_blocks": -5})
    coverage = ALL_ANSWERED | {"node1": 0.6}
    strict = netbench.check_validity(result, coverage)
    assert not strict["valid"]
    exempt = netbench.check_validity(result, coverage, frozenset({"node1", "node2"}))
    assert exempt["valid"]
    assert not any("node1" in r or "node2" in r for r in exempt["reasons"])
    only_one = netbench.check_validity(result, coverage, frozenset({"node1"}))
    assert any("node2 decided no blocks" in r for r in only_one["reasons"])


def test_load_stats_carry_the_failovers(tmp_path: Path):
    state = netbench.LoadState()
    state.failovers = 4
    write_load_files(tmp_path, state)
    assert netbench.load_stats(tmp_path, [], [], 0.0, 1.0)["submit_failovers"] == 4
    meta = netbench.read_json(tmp_path / "load-meta.json")
    del meta["submit_failovers"]
    (tmp_path / "load-meta.json").write_text(json.dumps(meta))
    assert netbench.load_stats(tmp_path, [], [], 0.0, 1.0)["submit_failovers"] == 0


def test_load_under_faults_end_to_end(tmp_path: Path):
    """Node0, a query node, dies for good at 20 s, validator node3 restarts at 40 s and query
    node1 is wiped at 60 s: no tx is lost and no payload goes missing."""
    events = {"node0": 20.0, "node3": 40.0, "node1": 60.0}
    done: set[str] = set()

    def faults(cluster: fakes.FakeCluster, now: float) -> None:
        for name, at in events.items():
            if now >= at and name not in done:
                done.add(name)
                {
                    "node0": cluster.kill,
                    "node3": cluster.restart,
                    "node1": cluster.wipe,
                }[name](name)
                if name == "node1":
                    cluster.start(name)

    run = run_cluster_load(
        tmp_path,
        80.0,
        [f"node{i}" for i in range(6)],
        {"node0", "node1"},
        faults,
        catchup_blocks_s=500.0,
    )
    assert done == set(events)
    assert run.load.txs
    assert included(run.load.txs)
    assert run.load.meta["submit_failovers"] > 0
    assert run.load.meta["submit_errors"] == 0
    assert run.load.meta["missing_payloads"] == []
