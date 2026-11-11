import importlib.util
import math
import subprocess
import sys
from fractions import Fraction
from pathlib import Path
from types import ModuleType
from typing import Any

import pytest

SCRIPT = Path(__file__).parents[1] / "vid-race-model.py"


def load_model() -> ModuleType:
    """The hyphenated script as a module, registered before it runs: its dataclasses look
    their module up in `sys.modules`."""
    spec = importlib.util.spec_from_file_location("vid_race_model", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


model: Any = load_model()


def scenario(**overrides: Any) -> Any:
    fields: dict[str, Any] = {
        "profile": "decaf-2025",
        "n": 10,
        "stake": "equal",
        "policy": "registration",
        "threshold": Fraction(1, 3),
        "parity": "full",
        "payload_mb": 100.0,
        "bw_mb_s": 1800.0,
        "concat_ms_per_mb": 0.5,
        "decode_ms_per_mb": 0.5,
        "intra": True,
    }
    return model.Scenario(**(fields | overrides))


@pytest.mark.parametrize("policy", model.POLICIES)
@pytest.mark.parametrize("threshold", [Fraction(1, 3), Fraction(2, 3)])
@pytest.mark.parametrize("parity", ["full", "none"])
def test_a_complete_systematic_set_is_never_ready_before_the_decode_threshold(
    policy, threshold, parity
):
    """The systematic range is the recovery threshold, so once every holder of it has
    delivered, the replica already holds a threshold of shares: at equal reconstruction
    cost the concat path never finishes first."""
    out = model.simulate(scenario(policy=policy, threshold=threshold, parity=parity))
    assert len(out.decode_finish) == 10 * 9
    assert all(
        d <= c for d, c in zip(out.decode_finish, out.concat_finish, strict=True)
    )
    assert out.race_quorum == out.decode_quorum


def test_the_race_saves_at_most_the_reconstruction_cost_gap():
    out = model.simulate(scenario(concat_ms_per_mb=0.37, decode_ms_per_mb=0.51))
    gap = (0.51 - 0.37) * 100.0
    savings = [d - c for d, c in zip(out.decode_finish, out.concat_finish, strict=True)]
    assert 0 < max(savings) <= gap + 1e-9


def test_the_leader_egress_is_the_mean_over_every_node_as_leader():
    sc = scenario(stake="top:1:0.2")
    weights = model.vid_weights(model.stake_fractions(sc.stake, sc.n, sc.profile, []))
    coded_mb = sc.payload_mb * sum(weights) / math.ceil(sum(weights) * sc.threshold)
    out = model.simulate(sc)
    assert out.leader_egress_mb == pytest.approx(coded_mb * (sc.n - 1) / sc.n)


def test_the_docstring_example_prints_one_row_per_policy():
    result = subprocess.run(
        [
            sys.executable,
            str(SCRIPT),
            *("--profile", "decaf-2025", "-n", "10", "--stake", "equal"),
            *("--policy", "registration", "nearest"),
        ],
        capture_output=True,
        text=True,
        check=True,
    )
    lines = result.stdout.splitlines()
    assert lines[0] == model.HEADER
    rows = [line.split(" | ") for line in lines[2:]]
    assert [row[5] for row in rows] == ["registration", "nearest"]
