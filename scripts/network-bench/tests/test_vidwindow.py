import vidwindow


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
    d, assignment = vidwindow.delays("decaf-2025", 10)
    assert assignment[0] == "eu-central-1"
    views = 60
    sim = vidwindow.Sim(10, 100e9 / 8, 1e6, views, d, k=4, leaders=[0] * (views + 3))
    sim.run()
    assert abs(vidwindow.pct(sim.view_times(15, 55), 0.5) - 158.0) < 1.0
    crit = [c for _, c in sim.critical(15, 55)]
    assert 148.0 < vidwindow.pct(crit, 0.5) < 154.0


def test_best_case_formula():
    wmin, gbps_per_mb = vidwindow.best_case(153.0, 4.6, 100, 34)
    assert abs(wmin - 78.8) < 0.1
    assert abs(gbps_per_mb - 0.314) < 0.001
