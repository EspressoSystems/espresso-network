import statistics

import numpy as np
import vidformula
import vidwindow
from vidwindow import C8G_4XLARGE, ZERO

N, K, Q = 10, 4, 7


def both(profile, mb, gbps, compute=ZERO, lane=False, views=160, seed=1):
    d, _ = vidwindow.delays(profile, N)
    sim = vidwindow.Sim(
        N,
        gbps * 1e9 / 8,
        mb * 1e6,
        views,
        d,
        k=K,
        compute=compute,
        seed=seed,
        lane=lane,
    )
    sim.run()
    rec = vidformula.run_views(N, K, d, mb, gbps, sim.leader, views, compute, lane)
    return sim, rec, views // 4, views - 5


def test_window_is_one_transfer_plus_the_detour_where_transfers_bind():
    # 20 MB at 2 Gbps: T = 180 ms, far above any delay. With distinct consecutive leaders
    # the sim's window is never below T + delta_j and equals it to the millisecond three
    # quarters of the time; the rest wait on a proposal that crossed behind more of the
    # leader's queue, by whole cross-region hops.
    d, _ = vidwindow.delays("decaf-2025", N)
    sim, _, lo, hi = both("decaf-2025", 20.0, 2.0)
    t_ms = 9 / 4 * 20 * 8 / 2
    checked = exact = 0
    for v in range(lo, hi):
        lead, nxt = sim.leader[v], sim.leader[v + 1]
        if lead == nxt or lead == sim.leader[v - 1]:
            continue
        delta = vidformula.detour(d, lead, nxt, K)
        for j in range(N):
            if j in (lead, nxt):
                continue
            window = (sim.pv[(j, v + 1)] - sim.kth[(j, v)]) * 1000.0
            assert window > t_ms + delta[j] - 0.5
            exact += abs(window - (t_ms + delta[j])) < 0.5
            checked += 1
    assert checked > 500
    assert exact / checked > 0.7


def test_quorum_round_trip_and_finality_read_off_the_matrix():
    d, assignment = vidwindow.delays("decaf-2025", N)
    assert assignment[0] == "eu-central-1"
    assert abs(vidformula.quorum_rtt(d, 0, 0, Q) - 158.0) < 0.1
    # The sim's 237 ms for leader 0 with no compute (test_vidwindow), 260.9 with compute.
    assert abs(vidformula.finality_latency_bound(d, 0, Q, 1.0, ZERO) - 237.0) < 0.1
    assert (
        abs(vidformula.finality_latency_bound(d, 0, Q, 1.0, C8G_4XLARGE) - 260.9) < 0.1
    )


def test_recurrence_matches_the_sim_where_latency_binds():
    sim, rec, lo, hi = both("decaf-2025", 1.0, 100.0, C8G_4XLARGE)
    for name in ("view_times", "finality", "windows"):
        s, f = getattr(sim, name)(lo, hi), getattr(rec, name)(lo, hi)
        assert abs(vidwindow.pct(s, 0.5) - vidwindow.pct(f, 0.5)) < 10.0, name
    assert (
        abs(
            statistics.mean(sim.view_times(lo, hi))
            - statistics.mean(rec.view_times(lo, hi))
        )
        < 0.5
    )
    assert vidwindow.pct(sim.finality(lo, hi), 0.5) == vidwindow.pct(
        rec.finality(lo, hi), 0.5
    )
    assert rec.late_pct(lo, hi) == sim.late_pct(lo, hi) == 0.0


def test_recurrence_matches_the_sim_where_bandwidth_binds():
    # The mean view agrees within 4 percent; the fluid sim's flows to idle receivers finish
    # first, so its windows and finality are shorter, within a fifth.
    for profile, mb, lane in (
        ("off", 92.0, False),
        ("decaf-2025", 49.0, False),
        ("off", 92.0, True),
    ):
        sim, rec, lo, hi = both(profile, mb, 15.0, C8G_4XLARGE, lane)
        s_view, f_view = (
            statistics.mean(sim.view_times(lo, hi)),
            statistics.mean(rec.view_times(lo, hi)),
        )
        assert abs(f_view - s_view) / s_view < 0.04, (profile, mb, lane)
        s_fin, f_fin = (
            vidwindow.pct(sim.finality(lo, hi), 0.5),
            vidwindow.pct(rec.finality(lo, hi), 0.5),
        )
        assert 0.97 < f_fin / s_fin < 1.2, (profile, mb, lane)
        assert (
            abs(
                vidwindow.pct(sim.windows(lo, hi), 0.5)
                - vidwindow.pct(rec.windows(lo, hi), 0.5)
            )
            < 15.0
        )


def test_lane_leaves_one_transfer_and_the_decode():
    _, rec, lo, hi = both("off", 92.0, 15.0, C8G_4XLARGE, lane=True)
    assert abs(statistics.mean(rec.view_times(lo, hi)) - (110.4 + 67.2)) < 1.0
    assert rec.late_pct(lo, hi) > 99.0


def test_point_reports_what_the_sim_reports():
    formula = vidformula.point("decaf-2025", N, 15.0, 20.0, K, 60, compute=C8G_4XLARGE)
    sim = vidwindow.point("decaf-2025", N, 15.0, 20.0, K, 60, compute=C8G_4XLARGE)
    assert set(sim) <= set(formula)
    assert abs(formula["view_p50"] - sim["view_p50"]) < 5.0


def test_pairs_table_covers_every_region_pair():
    rows = vidformula.pairs("decaf-2025", N, K, 20.0, 15.0, C8G_4XLARGE, False)
    regions = {r["leader"] for r in rows}
    assert {r["next"] for r in rows} == regions
    assert sum(r["n"] for r in rows) == N * (N - 1)
    eu = next(r for r in rows if r["leader"] == r["next"] == "eu-central-1")
    assert eu["q_ms"] == 158.0 and eu["delta_p50"] == 0.0
    assert all(np.isfinite(r["view_ms"]) for r in rows)
