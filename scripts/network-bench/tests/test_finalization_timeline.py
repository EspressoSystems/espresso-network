import xml.etree.ElementTree as ET
from pathlib import Path

import finalization_timeline as ft
import pytest
from leadertrace import Trace

MS = 1_000_000


def timeline_trace(split: bool) -> Trace:
    """Nodes 0 and 1 lead views 1 and 2, node 2 view 0. View 1 is queued at 1000 ms, view 0 at
    800 ms: a cycle of 200 ms."""
    trace: Trace = {0: {}, 1: {}, 2: {}}
    trace[2][0] = {"proposal_queued": 800 * MS}
    leader = {
        "proposal_queued": 1000 * MS,
        "request_block_header_queued": 940 * MS,
        "block_built_applied": 990 * MS,
        "ns_disperse_start": 960 * MS,
        "vid_shares_unicast_end": 985 * MS,
    }
    if split:
        leader |= {"ns_disperse_end": 970 * MS, "vid_shares_unicast_start": 975 * MS}
    trace[0][1] = leader
    trace[0][0] = {"cert1_v_minus_1_input_dispatched": 1005 * MS}
    for node, offset in enumerate((10, 20, 30)):
        trace[node].setdefault(1, {})["proposal_validated_v_minus_1"] = (
            1000 + offset
        ) * MS
    trace[1][2] = {"proposal_queued": 1200 * MS}
    return trace


def test_view_rows_are_relative_to_the_proposal_and_take_the_cluster_median():
    (row,) = ft.view_rows(timeline_trace(split=False), [1])
    assert row["cycle"] == 200
    assert row["request_block"] == -60
    assert row["vid_unicast_end"] == -15
    assert row["proposal_recv"] == 20
    assert row["cert1_at_L_V"] == 5
    assert row["vid_disperse"] is None
    assert row["cert2_formed"] is None


def test_metrics_are_medians_that_skip_absent_values():
    rows: list[ft.ViewRow] = [
        {"cycle": 100.0, "cert1_formed": 10.0},
        {"cycle": 300.0, "cert1_formed": None},
        {"cycle": 200.0, "cert1_formed": 30.0},
    ]
    rows = [{**dict.fromkeys(ft.EVENTS), **row} for row in rows]
    m = ft.timeline_metrics(rows)
    assert (m["n_views"], m["cycle"], m["cert1_formed"]) == (3, 200, 20)
    assert m["vote1_sent"] == 0
    assert m["finality_ms"] == 10
    assert m["throughput_vps"] == pytest.approx(5)


def test_metrics_split_recover_into_decode_and_commit():
    row = dict.fromkeys(ft.EVENTS) | {
        "cycle": 100.0,
        "vid_threshold_reached": 50.0,
        "recover_decode_end": 80.0,
        "block_reconstructed": 90.0,
    }
    m = ft.timeline_metrics([row])
    assert (m["recover_decode_ms"], m["recover_commit_ms"]) == (30, 10)
    row["recover_decode_end"] = None
    m = ft.timeline_metrics([row])
    assert (m["recover_decode_ms"], m["recover_commit_ms"]) == (40, 0)


def test_metrics_need_views():
    with pytest.raises(ValueError, match="no views"):
        ft.timeline_metrics([])


@pytest.mark.parametrize(
    ("split", "labels"),
    [
        (True, ["build/prep", "dispersal", "share-sign", "fan-out", "header"]),
        (False, ["build/prep", "disperse + fan-out", "header"]),
    ],
)
def test_build_bars_split_only_with_the_events_that_split_them(
    split: bool, labels: list[str]
):
    trace = timeline_trace(split)
    m = ft.timeline_metrics(ft.view_rows(trace, [1]))
    assert [label for label, *_ in ft.build_segments(m)] == labels


def test_svg_labels_the_combined_bar_and_parses(tmp_path: Path):
    out = tmp_path / "timeline.svg"
    ft.write_timeline(timeline_trace(split=False), [1], out)
    root = ET.fromstring(out.read_bytes())
    assert root.tag.endswith("svg")
    assert "per-view cycle = 200 ms" in out.read_text(encoding="utf-8")


def test_svg_renders_without_any_cluster_events(tmp_path: Path):
    trace: Trace = {
        0: {0: {"proposal_queued": 0}, 1: {"proposal_queued": 100 * MS}},
    }
    out = tmp_path / "timeline.svg"
    ft.write_timeline(trace, [1], out)
    ET.fromstring(out.read_bytes())
