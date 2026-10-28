import nullserver
import pytest


@pytest.mark.timeout(30)
@pytest.mark.parametrize("scan_processes", [0, 1])
def test_selftest_submits_at_the_rate_and_finds_every_tx_in_the_null_blocks(
    scan_processes,
):
    """20 txs/s of 1000 bytes for 2 s; scans in a process exercise the production wiring."""
    result = nullserver.run_selftest(
        0.02, 2, workers=2, tx_size=1000, scan_processes=scan_processes
    )
    assert result.submitted_mb_s == pytest.approx(0.02, rel=0.1)
    assert result.sent >= 30
    assert result.included == result.sent
