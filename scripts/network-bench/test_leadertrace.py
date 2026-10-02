from collections import Counter
from pathlib import Path

import leadertrace
import pytest

MS = 1_000_000
HEADER = "view,node_id,event,ts_ns\n"


def write_trace(path: Path, node: int, rows: list[tuple[int, str, int]]) -> Path:
    path.write_text(
        HEADER + "".join(f"{view},{node},{event},{ts}\n" for view, event, ts in rows)
    )
    return path


# (event, ms after t0) of a leader's view; t0 is the previous view's proposal_queued
PATH_EVENTS = (
    ("request_block_header_queued", 2),
    ("block_built_applied", 4),
    ("header_created_applied", 5),
    ("proposal_queued", 10),
    ("proposal_broadcast_start", 11),
    ("proposal_broadcast_end", 13),
    ("proposal_validated_v_minus_1", 15),
    ("vote1_broadcast_start", 16),
    ("cert1_v_minus_1_input_dispatched", 20),
    ("vote2_v_minus_1_broadcast_end", 22),
    ("cert2_v_minus_1_input_dispatched", 27),
    ("leaf_decided", 40),
)
PATH_DURATIONS = [2, 2, 1, 5, 1, 2, 2, 1, 4, 2, 5, 13]


def leader_rows(view: int) -> list[tuple[int, str, int]]:
    return [(view, event, (10 * view + off) * MS) for event, off in PATH_EVENTS]


def three_nodes() -> leadertrace.Trace:
    """Views 0..3, leader of view v is node v % 3; views 1..3 have a t0 anchor."""
    trace: leadertrace.Trace = {0: {}, 1: {}, 2: {}}
    for view in range(4):
        for _, event, ts in leader_rows(view):
            trace[view % 3].setdefault(view, {})[event] = ts
    return trace


def test_read_traces_groups_by_node_view_event(tmp_path: Path):
    rows = [(1, "proposal_queued", 100), (1, "leaf_decided", 200)]
    paths = [
        write_trace(tmp_path / "a.csv", 0, rows),
        write_trace(tmp_path / "b.csv", 3, [(2, "proposal_queued", 300)]),
    ]
    assert leadertrace.read_traces(paths) == {
        0: {1: {"proposal_queued": 100, "leaf_decided": 200}},
        3: {2: {"proposal_queued": 300}},
    }


def test_read_traces_keeps_the_first_duplicate(tmp_path: Path):
    path = write_trace(
        tmp_path / "a.csv", 0, [(1, "leaf_decided", 100), (1, "leaf_decided", 50)]
    )
    assert leadertrace.read_traces([path]) == {0: {1: {"leaf_decided": 100}}}


@pytest.mark.parametrize(
    ("body", "line"),
    [
        ("1,0,leaf_decided,notanint\n", 2),
        ("1,0,leaf_decided\n", 2),
        ("1,0,leaf_decided,5\nx,0,leaf_decided,5\n", 3),
        ("1,0,leaf_decided,5,extra\n", 2),
    ],
)
def test_read_traces_names_file_and_line_of_a_malformed_row(
    tmp_path: Path, body: str, line: int
):
    path = tmp_path / "leader_trace_node0.csv"
    path.write_text(HEADER + body)
    with pytest.raises(ValueError, match=rf"{path.name}:{line}\b"):
        leadertrace.read_traces([path])


def test_read_traces_rejects_a_wrong_header(tmp_path: Path):
    path = tmp_path / "t.csv"
    path.write_text("a,b,c,d\n1,0,x,5\n")
    with pytest.raises(ValueError, match="t.csv:1"):
        leadertrace.read_traces([path])


def test_leaders_maps_view_to_the_proposing_node():
    assert leadertrace.leaders(three_nodes()) == {
        0: (0, 10 * MS),
        1: (1, 20 * MS),
        2: (2, 30 * MS),
        3: (0, 40 * MS),
    }


def test_leaders_rejects_two_proposers_for_one_view():
    trace = three_nodes()
    trace[1][0] = {"proposal_queued": 5}
    with pytest.raises(ValueError, match=r"view 0 .* 0 and 1"):
        leadertrace.leaders(trace)


def test_leader_path_splits_a_view_at_each_event():
    path, skipped = leadertrace.leader_path(three_nodes(), warmup=1)
    assert skipped == Counter()
    assert list(path) == [1, 2, 3]
    assert path[2] == pytest.approx(PATH_DURATIONS)


def test_leader_path_sums_to_decision_time_after_t0():
    path, _ = leadertrace.leader_path(three_nodes(), warmup=1)
    for row in path.values():
        assert sum(row) == pytest.approx(40)


def test_leader_path_drops_the_warmup_views():
    path, _ = leadertrace.leader_path(three_nodes(), warmup=3)
    assert list(path) == [3]


def test_leader_path_keeps_negative_durations():
    trace = three_nodes()
    view, t0_ms, skew_ms = 2, 10, 2
    cert1_ms = t0_ms * view + dict(PATH_EVENTS)["cert1_v_minus_1_input_dispatched"]
    trace[view % 3][view]["vote2_v_minus_1_broadcast_end"] = (cert1_ms - skew_ms) * MS
    path, skipped = leadertrace.leader_path(trace, warmup=view)
    segment = [name for name, _ in leadertrace.LEADER_PATH].index("Cert1 -> vote2 sent")
    assert skipped == Counter()
    assert path[view][segment] == pytest.approx(-skew_ms)


def test_leader_path_counts_skips_by_reason():
    trace = three_nodes()
    del trace[1][1]["header_created_applied"]
    del trace[2][2]["cert2_v_minus_1_input_dispatched"]
    del trace[0][3]["leaf_decided"]
    path, skipped = leadertrace.leader_path(trace, warmup=0)
    assert list(path) == []
    assert skipped == Counter(
        {
            "no t0 anchor": 1,
            "missing header_created_applied": 1,
            "missing cert2_v_minus_1_input_dispatched": 1,
            "missing leaf_decided": 1,
        }
    )


def test_view_times_gives_proposal_queued_in_unix_seconds():
    trace = {0: {7: {"proposal_queued": 1_500_000_000}}}
    assert leadertrace.view_times(trace) == {7: pytest.approx(1.5)}


def test_cluster_finality_takes_the_quorum_decision():
    trace = three_nodes()
    # N = 3, quorum 3: the slowest node decides it
    for node, offset in zip(range(3), (50, 40, 90)):
        trace[node].setdefault(1, {})["leaf_decided"] = (20 + offset) * MS
    assert leadertrace.cluster_finality_ms(trace, warmup=1)[0][1] == pytest.approx(90)


def test_cluster_finality_quorum_counts_nodes_that_reported_nothing():
    trace: leadertrace.Trace = {
        n: {5: {"leaf_decided": (100 + n) * MS}} for n in range(3)
    }
    trace[0][5]["proposal_queued"] = 90 * MS
    trace[3] = {6: {}}
    # N = 4, quorum 3: the 3rd decision at 102 ms
    finality, skipped = leadertrace.cluster_finality_ms(trace, warmup=0)
    assert finality == {5: pytest.approx(12)}
    assert skipped == Counter()
    del trace[2][5]
    assert leadertrace.cluster_finality_ms(trace, warmup=0) == (
        {},
        Counter({"below quorum": 1}),
    )


def decided_everywhere() -> leadertrace.Trace:
    trace = three_nodes()
    for views in trace.values():
        for view in range(4):
            views.setdefault(view, {})["leaf_decided"] = (10 * view + 50) * MS
    return trace


def test_cluster_finality_skips_views_nobody_decided():
    trace = decided_everywhere()
    for views in trace.values():
        del views[1]["leaf_decided"]
    finality, skipped = leadertrace.cluster_finality_ms(trace, warmup=0)
    assert 1 not in finality
    assert skipped == Counter({"no decision": 1})


def test_cluster_finality_counts_decisions_of_views_without_a_leader():
    trace = decided_everywhere()
    trace[0][99] = {"leaf_decided": 5}
    _, skipped = leadertrace.cluster_finality_ms(trace, warmup=0)
    assert skipped == Counter({"no leader": 1})
