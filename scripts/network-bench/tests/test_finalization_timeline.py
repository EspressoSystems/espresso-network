import xml.etree.ElementTree as ET
from pathlib import Path

import finalization_timeline as ft
import pytest
from leadertrace import Trace

MS = 1_000_000


def timeline_trace(layout: str) -> Trace:
    """Nodes 0 and 1 lead views 1 and 2, node 2 view 0. View 1 is queued at 1000 ms, view 0 at
    800 ms: a cycle of 200 ms."""
    trace: Trace = {0: {}, 1: {}, 2: {}}
    trace[2][0] = {"proposal_queued": 800 * MS}
    leader = {
        "proposal_queued": 1000 * MS,
        "request_block_header_queued": 940 * MS,
        "block_built_applied": 960 * MS,
        "header_created_applied": 962 * MS,
        "proposal_assembled": 997 * MS,
        "vid_shares_unicast_start": 975 * MS,
        "vid_shares_unicast_end": 985 * MS,
    }
    if layout == "built":
        leader |= {
            "block_request_started": 940 * MS + MS // 2,
            "block_payload_start": 941 * MS,
            "block_commitments_start": 942 * MS,
            "block_commitments_end": 958 * MS,
            "vid_fragments_start": 960 * MS,
            "vid_fragments_end": 970 * MS,
        }
    else:
        leader |= {"ns_disperse_start": 960 * MS, "ns_disperse_end": 970 * MS}
    trace[0][1] = leader
    trace[0][0] = {"cert1_v_minus_1_obtained": 1005 * MS}
    for node, offset in enumerate((10, 20, 30)):
        trace[node].setdefault(1, {})["proposal_validated_v_minus_1"] = (
            1000 + offset
        ) * MS
    trace[1][2] = {"proposal_queued": 1200 * MS}
    for node, offset in enumerate((40, 50, 60)):
        trace[node][1]["leaf_decided"] = (1000 + offset) * MS
    return trace


def test_view_rows_are_relative_to_the_proposal_and_take_the_cluster_median():
    (row,) = ft.view_rows(timeline_trace("encode"), [1])
    assert row["cycle"] == 200
    assert row["request_block"] == -60
    assert row["vid_unicast_end"] == -15
    assert row["proposal_validated"] == 20
    assert row["cert1_at_L_V"] == 5
    assert row["commitments_end"] is None
    assert row["cert2_formed"] is None
    assert row["leaf_decided"] == 50


def test_metrics_are_medians_that_skip_absent_values():
    rows: list[ft.ViewRow] = [
        {"cycle": 100.0, "cert1_formed": 10.0},
        {"cycle": 300.0, "cert1_formed": None},
        {"cycle": 200.0, "cert1_formed": 30.0},
    ]
    rows = [{**dict.fromkeys(ft.EVENTS), **row} for row in rows]
    m = ft.timeline_metrics(rows)
    assert (m["n_views"], m["cycle"], m["cert1_formed"]) == (3, 200, 20)
    assert m["vote1_arrived"] == 0
    assert m["finality_ms"] == 0
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
    ("layout", "rows_labels"),
    [
        (
            "built",
            [
                (0, "request queue + tx select"),
                (0, "builder start + empty-block delay"),
                (0, "build/prep"),
                (1, "encode + commitments"),
                (1, "sign + fragments"),
                (2, "fan-out (serialize + enqueue)"),
                (0, "parent state + header"),
                (0, "parent Cert1 wait"),
                (0, "commit + sign"),
            ],
        ),
        (
            "encode",
            [
                (0, "build/prep"),
                (1, "dispersal"),
                (2, "fan-out (serialize + enqueue)"),
                (0, "parent state + header"),
                (0, "parent Cert1 wait"),
                (0, "commit + sign"),
            ],
        ),
    ],
)
def test_build_segments_follow_the_path_of_the_trace(
    layout: str, rows_labels: list[tuple[int, str]]
):
    m = ft.timeline_metrics(ft.view_rows(timeline_trace(layout), [1]))
    segments = ft.build_segments(m)
    assert [(row, label) for row, label, *_ in segments] == rows_labels


def test_build_segments_header_follows_block_built_and_ends_at_the_proposal():
    m = ft.timeline_metrics(ft.view_rows(timeline_trace("built"), [1]))
    by_label = {label: (start, end) for _, label, start, end, _ in ft.build_segments(m)}
    assert by_label["parent state + header"] == ("block_built", "header_created")
    assert by_label["commit + sign"][1] == "proposal_queued"
    assert m["proposal_queued"] == 0


def test_svg_labels_the_built_bars(tmp_path: Path):
    out = tmp_path / "timeline.svg"
    ft.write_timeline(timeline_trace("built"), [1], out)
    text = out.read_text(encoding="utf-8")
    assert "encode + commitments 16" in text
    assert "sign + fragments" in text


@pytest.mark.parametrize(
    ("layout", "expected"),
    [
        (
            "built",
            [
                ("request_started", "payload_start"),
                ("payload_start", "block_built"),
            ],
        ),
        ("encode", [("request_block", "block_built")]),
    ],
)
def test_empty_block_delay_splits_build_prep_on_the_built_layout(
    layout: str, expected: list[tuple[str, str]]
):
    m = ft.timeline_metrics(ft.view_rows(timeline_trace(layout), [1]))
    row0 = [
        seg
        for seg in ft.build_segments(m)
        if seg[1] in ("builder start + empty-block delay", "build/prep")
    ]
    assert [(start, end) for _, _, start, end, _ in row0] == expected


def test_frame_stacks_rows_inside_build_lanes_only():
    f = ft.Frame.of(ft.timeline_metrics(ft.view_rows(timeline_trace("built"), [1])))
    rows = [f.bar_y("L_V", row) for row in range(3)]
    assert rows == sorted(rows) and len(set(rows)) == 3
    assert rows[2] + f.bar_height("L_V") <= f.top("L_V") + f.lane_height("L_V")
    assert f.top("NextRX") == f.top("L_V") + f.build_lane_h
    assert f.plot_bottom == f.top("Deciding") + f.lane_h


def test_bar_places_labels_that_do_not_fit_and_never_drops_them():
    f = ft.Frame(t_min=0, t_max=1000)
    for outside in (None, "right", "above", "below"):
        assert "tiny" in ft.bar(f, 0, 1, 10, 20, "#000", "tiny", 9, outside=outside)
    assert 'text-anchor="start"' in ft.bar(
        f, 0, 1, 10, 20, "#000", "tiny", 9, outside="right"
    )
    assert 'text-anchor="middle"' in ft.bar(f, 0, 1, 10, 20, "#000", "tiny", 9)


def test_build_bars_skip_bars_shorter_than_1_ms():
    trace = timeline_trace("built")
    trace[0][1]["proposal_assembled"] = 999_800_000
    m = ft.timeline_metrics(ft.view_rows(trace, [1]))
    f = ft.Frame.of(m)
    assert "commit + sign" not in ft.build_bars(m, f, 0.0, "L_V")
    assert "parent Cert1 wait" in ft.build_bars(m, f, 0.0, "L_V")


def test_svg_parses_and_labels_the_encode_bars_and_finality(tmp_path: Path):
    out = tmp_path / "timeline.svg"
    ft.write_timeline(timeline_trace("encode"), [1], out)
    root = ET.fromstring(out.read_bytes())
    assert root.tag.endswith("svg")
    text = out.read_text(encoding="utf-8")
    assert "dispersal 10" in text
    assert "fan-out (serialize + enqueue) 10" in text
    assert "share-sign" not in text
    assert "collect vote1 + Cert1  (205 ms)" in text
    assert "per-view cycle = 200 ms" in text
    assert "finality of V ≈ 50 ms" in text


def test_svg_renders_without_any_cluster_events(tmp_path: Path):
    trace: Trace = {
        0: {0: {"proposal_queued": 0}, 1: {"proposal_queued": 100 * MS}},
    }
    out = tmp_path / "timeline.svg"
    ft.write_timeline(trace, [1], out)
    ET.fromstring(out.read_bytes())


def build_row(layout: str = "built", **overrides: float | None) -> ft.ViewRow:
    row: ft.ViewRow = dict.fromkeys(ft.EVENTS)
    row |= {"cycle": 100.0, "request_block": -50.0, "block_built": -40.0}
    row |= {"vid_unicast_start": -20.0, "vid_unicast_end": -10.0}
    row |= {"header_created": -35.0, "proposal_assembled": -5.0}
    if layout == "built":
        row |= {"request_started": -49.5, "payload_start": -49.0}
        row |= {"commitments_start": -48.0, "commitments_end": -42.0}
        row |= {"fragments_start": -39.0, "fragments_end": -30.0}
    else:
        row |= {"ns_disperse_start": -40.0, "ns_disperse_end": -30.0}
    return row | overrides


def test_layout_follows_the_majority_and_medians_use_complete_views_only():
    rows = [
        build_row(),
        build_row(vid_unicast_end=-8.0),
        build_row("encode", vid_unicast_end=-100.0),
    ]
    m = ft.timeline_metrics(rows)
    assert m["fragments_end"] == -30
    assert m["vid_unicast_end"] == -9  # the Encode view is left out
    assert m["present_ns_disperse_end"] == 0
    rows = [build_row("encode"), build_row("encode"), build_row()]
    m = ft.timeline_metrics(rows)
    assert m["ns_disperse_end"] == -30
    assert m["present_fragments_end"] == 0


@pytest.mark.parametrize(
    "missing", ["commitments_end", "header_created", "payload_start", "request_started"]
)
def test_a_view_missing_a_layout_event_is_skipped(missing: str):
    m = ft.timeline_metrics([build_row(**{missing: None})])
    assert ft.build_segments(m) == []
    assert m["request_block"] == 0


def test_outside_labels_that_would_overlap_are_dropped():
    f = ft.Frame(t_min=0, t_max=1000)
    placed: list[tuple[float, float, int]] = []
    first = ft.bar(f, 0, 1, 10, 20, "#000", "first", 9, outside="right", placed=placed)
    second = ft.bar(
        f, 2, 3, 10, 20, "#000", "second", 9, outside="right", placed=placed
    )
    assert "first" in first and "second" not in second
    assert "<rect" in second
    other_row = ft.bar(
        f, 2, 3, 40, 20, "#000", "third", 9, outside="right", placed=placed
    )
    assert "third" in other_row
