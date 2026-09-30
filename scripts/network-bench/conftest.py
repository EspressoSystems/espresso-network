import os
from pathlib import Path

import pytest
from fakes import (
    FakeClock,
    FakeRegistry,
    FakeSystem,
    RunHarness,
    awsb,
    valid_result,
)


def pytest_collection_modifyitems(
    config: pytest.Config, items: list[pytest.Item]
) -> None:
    if "SLOW_TESTS" in os.environ:
        return
    skip = pytest.mark.skip(reason="slow: SLOW_TESTS unset")
    for item in items:
        if "slow" in item.keywords:
            item.add_marker(skip)


@pytest.fixture
def system() -> FakeSystem:
    return FakeSystem()


@pytest.fixture
def clock(system: FakeSystem) -> FakeClock:
    assert isinstance(system.clock, FakeClock)
    return system.clock


@pytest.fixture
def isolated(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Runs the test in `tmp_path`, so the relative `OUT_ROOT` lands there."""
    monkeypatch.chdir(tmp_path)
    return tmp_path


@pytest.fixture
def registry() -> FakeRegistry:
    return FakeRegistry("test/image", "v1", [("linux", "arm64")])


@pytest.fixture
def run_harness(isolated: Path, monkeypatch: pytest.MonkeyPatch) -> RunHarness:
    return RunHarness(monkeypatch)


@pytest.fixture
def valid(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(awsb, "write_report", lambda *a, **k: valid_result())
