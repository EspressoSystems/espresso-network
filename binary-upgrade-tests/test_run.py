import pytest

import run

LIVE = {"mainnet": "20260918", "decaf": "0.6.3.10"}
MAIN = {"20260814", "0.6.3.0", "0.7.0.0"}

MAINNET_REV = "fd221e8e4b51f5ecec0ae44bc99f6b7f82cd1171"

# Trimmed from https://query.main.net.espresso.network/v1/status/metrics
METRICS = f"""\
consensus_validate_and_apply_header_duration_count 544739
# HELP consensus_version version
# TYPE consensus_version gauge
consensus_version{{desc="20260918",rev="{MAINNET_REV}",timestamp="2026-09-18T00:00:54.000000000+05:00"}} 1
# HELP consensus_vid_disperse_duration seconds
# TYPE consensus_vid_disperse_duration histogram
"""


@pytest.mark.parametrize(
    "live, reachable, containing, bases",
    [
        pytest.param(LIVE, MAIN, set(), ["20260918", "0.6.3.10"], id="main"),
        pytest.param(
            LIVE,
            {"20260814", "0.6.3.0", "0.6.3.10"},
            set(),
            ["20260918", "0.6.3.10"],
            id="release-0.6.3",
        ),
        pytest.param(
            LIVE,
            {"20260814", "0.6.3.0", "0.6.3.10"},
            {"0.6.3.10"},
            ["20260918"],
            id="decaf-contains-head",
        ),
        pytest.param(
            LIVE, {"20260814", "20260918"}, set(), ["20260918"], id="legacy-branch"
        ),
        pytest.param(
            {"mainnet": "0.6.3.10", "decaf": "0.7.0.2"},
            {"20260814", "0.6.3.0"},
            set(),
            ["0.6.3.10"],
            id="decaf-above-head-line",
        ),
        pytest.param(
            {"mainnet": "0.6.3.10", "decaf": "0.6.3.10"},
            MAIN,
            set(),
            ["0.6.3.10"],
            id="dedupe",
        ),
    ],
)
def test_pick_base_tags(
    live: dict[str, str], reachable: set[str], containing: set[str], bases: list[str]
) -> None:
    assert run.pick_base_tags(live, reachable, containing) == bases


@pytest.mark.parametrize(
    "reachable, containing",
    [
        pytest.param(set(), set(), id="shallow"),
        pytest.param(MAIN, {"20260918", "0.6.3.10"}, id="all-excluded"),
    ],
)
def test_pick_base_tags_fails(reachable: set[str], containing: set[str]) -> None:
    with pytest.raises(RuntimeError):
        run.pick_base_tags(LIVE, reachable, containing)


def test_parse_consensus_version() -> None:
    assert run.parse_consensus_version(METRICS) == ("20260918", MAINNET_REV)


def test_parse_consensus_version_missing() -> None:
    with pytest.raises(RuntimeError):
        run.parse_consensus_version("consensus_vid_disperse_duration_count 1\n")


@pytest.mark.parametrize(
    "desc, commit",
    [
        pytest.param("main", MAINNET_REV, id="not-a-tag"),
        pytest.param("20260918", None, id="missing-locally"),
        pytest.param("20260918", "0" * 40, id="rev-mismatch"),
    ],
)
def test_check_live_tag_fails(desc: str, commit: str | None) -> None:
    with pytest.raises(RuntimeError, match="mainnet"):
        run.check_live_tag("mainnet", desc, MAINNET_REV, commit)


def test_check_live_tag() -> None:
    run.check_live_tag("mainnet", "20260918", MAINNET_REV, MAINNET_REV)


def test_tag_order() -> None:
    tags = ["0.6.3.10", "20260918", "0.6.3.9", "0.7.0.0"]
    assert sorted(tags, key=run.tag_order) == [
        "20260918",
        "0.6.3.9",
        "0.6.3.10",
        "0.7.0.0",
    ]
