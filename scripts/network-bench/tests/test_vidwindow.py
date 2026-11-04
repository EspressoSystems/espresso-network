import statistics

import vidwindow
from vidwindow import C8G_4XLARGE, ZERO, Compute, Cost

ROUND_ROBIN = [i % 10 for i in range(300)]


def sim(profile, mb, gbps, leaders, compute=ZERO, lane=False, views=120):
    d, _ = vidwindow.delays(profile, 10)
    s = vidwindow.Sim(
        10,
        gbps * 1e9 / 8,
        mb * 1e6,
        views,
        d,
        k=4,
        leaders=leaders,
        compute=compute,
        lane=lane,
    )
    s.run()
    return s, 30, views - 5


def test_without_latency_the_window_is_one_share_transfer():
    # 10 nodes, 20 MB, 10 Gbps: a share is 5 MB and goes to 9 peers, 36 ms per transfer.
    row = vidwindow.point(
        "off", 10, 10.0, 20.0, vidwindow.threshold(10, "f+1"), views=40
    )
    assert row["share_ms"] == 36.0
    assert abs(row["crit_p50"] - 36.0) < 1.0
    assert abs(row["view_p50"] - 72.0) < 1.0
    assert abs(row["crit_p10"] - row["crit_p90"]) < 0.2


def test_two_f_plus_one_halves_the_bytes():
    f1 = vidwindow.point(
        "off", 10, 10.0, 20.0, vidwindow.threshold(10, "f+1"), views=40
    )
    f2 = vidwindow.point(
        "off", 10, 10.0, 20.0, vidwindow.threshold(10, "2f+1"), views=40
    )
    assert vidwindow.threshold(10, "2f+1") == 7
    assert abs(f2["view_p50"] / f1["view_p50"] - 4 / 7) < 0.02


def test_under_latency_the_view_is_the_quorum_round_trip():
    # Every view led by node 0 in eu-central-1: the 7th vote comes from ap-southeast-1, two
    # 79 ms hops away, and the critical voter there has the view less its own 5 ms of slack.
    _, assignment = vidwindow.delays("decaf-2025", 10)
    assert assignment[0] == "eu-central-1"
    s, lo, hi = sim("decaf-2025", 1.0, 100.0, [0] * 200, views=60)
    assert abs(vidwindow.pct(s.view_times(lo, hi), 0.5) - 158.0) < 1.0
    crit = [c for _, c in s.critical(lo, hi)]
    assert 148.0 < vidwindow.pct(crit, 0.5) < 154.0
    # Cert1 forms at the far quorum node 79 ms after the votes left, its vote2 takes another
    # 79 ms to the quorum of deciders.
    assert abs(vidwindow.pct(s.finality(lo, hi), 0.5) - 237.0) < 1.0


def test_compute_adds_to_the_latency_chain():
    # Latency-bound: the view grows by the compute on the vote path, 24.4 ms on c8g.4xlarge,
    # and nothing waits on a decode.
    s, lo, hi = sim("decaf-2025", 1.0, 100.0, [0] * 200, C8G_4XLARGE, views=60)
    form = vidwindow.closed_form(10, 4, 1.0, 100.0, 158.0, 5.0, C8G_4XLARGE)
    assert abs(form["view_ms"] - 182.3) < 0.1
    assert abs(vidwindow.pct(s.view_times(lo, hi), 0.5) - form["view_ms"]) < 1.0
    assert s.late_pct(lo, hi) == 0.0


def test_in_order_delivery_costs_two_transfers_a_view():
    # 92 MB at 15 Gbps: T = 110 ms. The next leader's rebroadcast leaves once its vote1 is
    # out (verify 5 + vote1 11.9 ms), its dispersal behind that, and the proposal behind both,
    # so a view is two transfers plus the vote1 path; the build (103.6 ms) and the proposal's
    # persist hide under the transfers. Consecutive views alternate around the mean.
    s, lo, hi = sim("off", 92.0, 15.0, ROUND_ROBIN, C8G_4XLARGE)
    form = vidwindow.closed_form(10, 4, 92.0, 15.0, 0.0, 0.0, C8G_4XLARGE)
    assert abs(form["share_ms"] - 110.4) < 0.1
    assert abs(form["view_ms"] - (5.0 + 11.9 + 2 * 110.4)) < 0.2
    assert abs(statistics.mean(s.view_times(lo, hi)) - form["view_ms"]) < 3.0
    assert abs(vidwindow.pct(s.windows(lo, hi), 0.5) - form["window_ms"]) < 10.0
    # The proposal crosses a 0.1 ms link in a share's time.
    assert vidwindow.pct(s.transit(lo, hi), 0.5) > 100.0


def test_a_control_lane_leaves_one_transfer_and_the_decode():
    s, lo, hi = sim("off", 92.0, 15.0, ROUND_ROBIN, C8G_4XLARGE, lane=True)
    form = vidwindow.closed_form(10, 4, 92.0, 15.0, 0.0, 0.0, C8G_4XLARGE, lane=True)
    assert abs(form["view_ms"] - (110.4 + 67.2)) < 0.2
    assert abs(statistics.mean(s.view_times(lo, hi)) - form["view_ms"]) < 1.0
    assert s.late_pct(lo, hi) == 100.0


def test_a_slow_decode_delays_votes_and_the_view():
    # A 2 vCPU decode, 4 ms/MB: at 20 MB it outlasts the 48 ms window of T + vote1 and the
    # view stretches to about T + decode.
    slow = Compute(**{**C8G_4XLARGE.__dict__, "decode": Cost(0.0, 4.0)})
    s, lo, hi = sim("off", 20.0, 15.0, ROUND_ROBIN, slow)
    assert s.late_pct(lo, hi) > 90.0
    assert statistics.mean(s.view_times(lo, hi)) > 24.0 + 80.0 - 5.0


def test_point_reports_compute_and_finality():
    row = vidwindow.point("off", 10, 15.0, 20.0, 4, views=40, compute=C8G_4XLARGE)
    assert row["compute"]["decode"] == 14.6
    assert row["fin_p50"] > row["view_p50"]
    assert row["lane"] is False


def test_best_case_formula():
    wmin, gbps_per_mb = vidwindow.best_case(153.0, 4.6, 100, 34)
    assert abs(wmin - 78.8) < 0.1
    assert abs(gbps_per_mb - 0.314) < 0.001
