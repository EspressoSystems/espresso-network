import subprocess
from datetime import UTC, datetime, timedelta
from pathlib import Path

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    FAKE_EPOCH,
    FakeClock,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    awsb,
    completed,
    no_children,
    remote,
)

XANMOD = "6.17.0-x64v3-xanmod1"


def kernel_runner(release: str = XANMOD) -> FakeRunner:
    """Hosts that answer a new boot id once `systemctl reboot` ran on them."""
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    rebooted: set[str] = set()

    def target(argv: list[str]) -> str:
        return argv[-2]

    def reboot(argv: list[str]) -> subprocess.CompletedProcess:
        rebooted.add(target(argv))
        return completed()

    runner.respond("systemctl reboot", reboot)
    runner.respond(
        "boot_id",
        lambda argv: completed(stdout=f"boot{target(argv) in rebooted}\n"),
    )
    runner.respond("uname -r", lambda _: completed(stdout=f"{release}\n"))
    return runner


def xanmod_fleet(tmp_path: Path, runner: FakeRunner):
    clock = FakeClock(start=FAKE_EPOCH)
    fleet_dir = tmp_path.relative_to(Path.cwd())
    expires = datetime.fromtimestamp(FAKE_EPOCH, UTC) + timedelta(minutes=90)
    netbench.write_json(fleet_dir / "fleet.json", {"expires_at": expires.isoformat()})
    fleet = awsb.FleetState(
        FakeSystem(run=runner, clock=clock),
        awsb.RunConfig(tag="x", tcp_cc="bbr3"),
        fleet_dir,
        None,
        awsb.Interrupts(clock, no_children),
    )
    return fleet, remote(runner, tmp_path)


def targets(runner: FakeRunner, needle: str) -> set[str]:
    return {c[-2] for c in runner.calls if needle in c[-1]}


def test_xanmod_is_installed_on_node_hosts_only(isolated: Path):
    runner = kernel_runner()
    fleet, ssh = xanmod_fleet(isolated, runner)
    awsb.install_xanmod(fleet, ssh)
    ctl = f"{awsb.SSH_USER}@{ssh.hosts['ctl']['public_ip']}"
    nodes = {f"{awsb.SSH_USER}@{ssh.hosts[n]['public_ip']}" for n in ("node0", "node1")}
    assert targets(runner, "linux-xanmod-x64v3") == nodes
    assert targets(runner, "systemctl reboot") == nodes
    assert ctl not in {c[-2] for c in runner.calls}


def test_xanmod_rearms_the_shutdown_timer_until_expiry(isolated: Path):
    runner = kernel_runner()
    fleet, ssh = xanmod_fleet(isolated, runner)
    awsb.install_xanmod(fleet, ssh)
    assert runner.count("shutdown -P +90") == 2


def test_a_kernel_that_is_not_xanmod_fails(isolated: Path):
    runner = kernel_runner("6.8.0-1000-aws")
    fleet, ssh = xanmod_fleet(isolated, runner)
    with pytest.raises(awsb.RemoteError, match="not XanMod"):
        awsb.install_xanmod(fleet, ssh)
    assert not runner.ran("shutdown -P")


def test_a_host_that_never_reboots_times_out(isolated: Path):
    runner = kernel_runner()
    runner.table.insert(0, ("boot_id", lambda _: completed(stdout="boot\n")))
    fleet, ssh = xanmod_fleet(isolated, runner)
    with pytest.raises(awsb.RemoteError, match="not rebooted"):
        awsb.install_xanmod(fleet, ssh)


def test_bbr3_is_refused_on_arm64(isolated: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr(awsb, "caller_account", lambda *_: "acct")
    monkeypatch.setattr(awsb, "tools_on_path", lambda *_: None)
    monkeypatch.setattr(
        awsb, "instance_specs", lambda *_: ("arm64", {"c8g.2xlarge": 16384})
    )
    cfg = awsb.RunConfig(
        tag="x",
        nodes=2,
        load=netbench.BenchConfig(submit_nodes=1),
        tcp_cc="bbr3",
        latency="decaf-2025",
    )
    with pytest.raises(awsb.Refused, match="bbr3.*x86_64"):
        awsb.preflight(FakeSystem(run=FakeRunner()), cfg, awsb.plan_hosts(cfg))


def test_bbr3_adds_the_kernel_step_to_the_estimate():
    cfg = awsb.RunConfig(tag="x", latency="decaf-2025")
    plain = awsb.shot_seconds(cfg)
    bbr3 = awsb.shot_seconds(
        awsb.RunConfig(tag="x", latency="decaf-2025", tcp_cc="bbr3")
    )
    assert bbr3 == (
        plain[0] + awsb.KERNEL_EXPECTED_S,
        plain[1] + awsb.XANMOD_INSTALL_TIMEOUT_S + awsb.SSH_READY_TIMEOUT_S,
    )


def test_default_single_shot_installs_no_kernel(
    isolated: Path, monkeypatch: pytest.MonkeyPatch
):
    harness = FleetHarness(monkeypatch, isolated)
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert awsb.cmd_run(harness.single_shot_args(), FakeSystem(run=runner)) == 0
    assert not runner.ran("xanmod")


def test_xanmod_script_carries_the_archive_key(isolated: Path):
    """dl.xanmod.org redirects to gitlab.com, which served no key to the hosts."""
    runner = kernel_runner()
    fleet, ssh = xanmod_fleet(isolated, runner)
    awsb.install_xanmod(fleet, ssh)
    script = next(c[-1] for c in runner.calls if "linux-xanmod-x64v3" in c[-1])
    assert "BEGIN PGP PUBLIC KEY BLOCK" in script
    assert "dl.xanmod.org" not in script
