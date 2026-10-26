from pathlib import Path

import leader_window as lw
import numpy as np
import pytest
from leadertrace import Trace

MS = 1_000_000


def window() -> lw.Window:
    return lw.Window(1000 * MS, 1100 * MS)


def test_view_spans_split_the_disperse_only_with_its_events():
    full = {event: 0 for event in lw.SPLIT_EVENTS}
    assert lw.view_spans(full) == lw.SPANS
    labels = [span[2] for span in lw.view_spans({})]
    assert labels == [
        "block-build",
        "disperse + fan-out",
        "recover-decode",
        "txn-commit",
    ]
    only_end = {"ns_disperse_end": 0}
    assert lw.view_spans(only_end) == lw.view_spans({})


def test_window_clips_spans():
    w = window()
    assert w.clipped(990 * MS, 1010 * MS) == (0.0, 10.0)
    assert w.clipped(1090 * MS, 1200 * MS) == (90.0, 10.0)
    assert w.clipped(1110 * MS, 1200 * MS) is None
    assert w.clipped(1050 * MS, 1050 * MS) is None


def test_cpu_series_is_cores_busy_per_interval():
    cpu = [(t * MS, 10 * MS, 10_000) for t in (990, 1000, 1010, 1020)]
    xs, busy = lw.cpu_series(cpu, window())
    assert list(xs) == [0.0, 10.0, 20.0]  # the 990 interval ends before the window
    assert list(busy) == [1.0, 1.0, 1.0]  # 10 ms of CPU time in 10 ms


def test_cpu_series_is_none_without_an_interval_in_the_window():
    assert lw.cpu_series([], window()) is None
    assert lw.cpu_series([(1200 * MS, 10 * MS, 1)], window()) is None


def test_net_series_converts_bytes_to_gbps_per_interval():
    net = [(t * MS, 10 * MS, 125_000, 250_000) for t in (990, 1000, 1010)]
    xs, tx, rx = lw.net_series(net, window())
    assert list(xs) == [0.0, 10.0]
    assert list(tx) == pytest.approx([0.2, 0.2])  # 250 kB in 10 ms
    assert list(rx) == pytest.approx([0.1, 0.1])


def test_net_series_is_none_without_an_interval_in_the_window():
    assert lw.net_series([(10, 10, 1, 1)], window()) is None


def test_net_binned_of_a_constant_rate_is_that_rate():
    net = [(t * MS, 10 * MS, 125_000, 125_000) for t in range(0, 2200, 10)]
    xs, tx, rx = lw.net_binned(net, lw.Window(1000 * MS, 1200 * MS))
    assert list(xs) == [25.0, 75.0, 125.0, 175.0]
    assert tx == pytest.approx([0.1] * 4)
    assert np.allclose(tx, rx)


def test_cert1_stamps_are_row_v_of_the_next_leader():
    trace: Trace = {
        0: {5: {"proposal_queued": 500 * MS}},
        1: {
            5: {lw.CERT1: 590 * MS},
            6: {"proposal_queued": 600 * MS, lw.CERT1: 700 * MS},
        },
    }
    lead = {5: (0, 500 * MS), 6: (1, 600 * MS)}
    # Cert1(5) formed at L_6 just before proposal_queued(6), not Cert1(6) at 700 ms.
    assert lw.cert1_stamps(trace, lead, 5) == [590 * MS]


def leader_trace() -> tuple[Trace, dict[int, tuple[int, int]]]:
    trace: Trace = {0: {}}
    lead = {}
    for view in range(3, 8):
        queued = view * 100 * MS
        trace[0][view] = {
            "proposal_queued": queued,
            "request_block_header_queued": queued - 80 * MS,
            "header_created_applied": queued - 10 * MS,
            "leaf_decided": queued + 50 * MS,
        }
        lead[view] = (0, queued)
    return trace, lead


@pytest.mark.parametrize("with_samples", [False, True])
def test_figure_renders_with_and_without_samples(tmp_path: Path, with_samples: bool):
    trace, lead = leader_trace()
    cpu = (
        [(t * MS, 10 * MS, 6_000) for t in range(200, 900, 10)] if with_samples else []
    )
    net = (
        [(t * MS, 10 * MS, 1_000_000, 2_000_000) for t in range(200, 900, 10)]
        if with_samples
        else []
    )
    out = tmp_path / "leader_window.png"
    lw.write_leader_window(trace, lead, 5, cpu, net, out)
    assert out.read_bytes().startswith(b"\x89PNG")


def test_figure_needs_events_in_the_window(tmp_path: Path):
    trace, lead = leader_trace()
    trace[0] = {}
    with pytest.raises(ValueError, match="no events of views 3..7 on node 0"):
        lw.write_leader_window(trace, lead, 5, [], [], tmp_path / "x.png")
