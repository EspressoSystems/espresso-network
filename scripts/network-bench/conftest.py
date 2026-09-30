import os
from collections.abc import Iterator
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


@pytest.fixture(autouse=True)
def driver_log() -> Iterator[None]:
    """Undoes `setup_logging`, which rewires the module-global `aws-bench` logger."""
    log = awsb.log
    handlers, level, propagate = list(log.handlers), log.level, log.propagate
    yield
    for handler in [h for h in log.handlers if h not in handlers]:
        log.removeHandler(handler)
        handler.close()
    log.handlers[:] = handlers
    log.setLevel(level)
    log.propagate = propagate


@pytest.fixture
def system() -> FakeSystem:
    return FakeSystem()


@pytest.fixture
def clock(system: FakeSystem) -> FakeClock:
    assert isinstance(system.clock, FakeClock)
    return system.clock


@pytest.fixture
def isolated(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Runs the test in `tmp_path`, so the relative `OUT_ROOT` and the tofu plugin cache land
    there."""
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(awsb, "TF_PLUGIN_CACHE_DIR", tmp_path / "tf-plugins")
    return tmp_path


@pytest.fixture
def registry() -> FakeRegistry:
    return FakeRegistry("test/image", "v1", [("linux", "arm64")])


@pytest.fixture
def run_harness(isolated: Path, monkeypatch: pytest.MonkeyPatch) -> RunHarness:
    return RunHarness(monkeypatch, isolated)


@pytest.fixture
def valid(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(awsb, "write_report", lambda *a, **k: valid_result())
