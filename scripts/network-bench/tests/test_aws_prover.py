import dataclasses
import re
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    DOTENV_TEXT,
    MEMORY_MIB,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    RunHarness,
    Scripted,
    aws_manifest,
    awsb,
    fake_image,
    fake_images,
    fleet,
    remote,
)
from test_aws_plan import START_SH_GOLDEN_DIR, host, small_cfg

HERE = Path(__file__).parents[1]
LC_ADDRESS = "0x0000000000000000000000000000000000000004"


def config(*flags: str) -> Any:
    return awsb.config_from_args(awsb.parse_args(["run", "--tag", "x", *flags]))


@pytest.mark.parametrize(
    ("flags", "epoch_height", "ctl_type", "prover"),
    [
        ((), None, awsb.CTL_TYPE, False),
        (("--chaos",), 300, awsb.CHAOS_CTL_TYPE, False),
        (("--prover",), awsb.SHORT_EPOCH_HEIGHT, awsb.PROVER_CTL_TYPE, True),
        (("--chaos", "--prover"), awsb.SHORT_EPOCH_HEIGHT, awsb.PROVER_CTL_TYPE, True),
        (("--prover", "--epoch-height", "3000"), 3000, awsb.PROVER_CTL_TYPE, True),
        (("--prover", "--ctl-type", "c8g.8xlarge"), 300, "c8g.8xlarge", True),
        (("--chaos", "--prover", "--ctl-type", "c8g.xlarge"), 300, "c8g.xlarge", True),
        (("--epoch-height", "3000"), 3000, awsb.CTL_TYPE, False),
    ],
)
def test_prover_defaults(flags, epoch_height, ctl_type, prover):
    cfg = config(*flags)
    assert (cfg.epoch_height, cfg.ctl_type, cfg.prover) == (
        epoch_height,
        ctl_type,
        prover,
    )
    assert awsb.staking(cfg) == (epoch_height is not None)


def test_the_stake_table_capacity_is_200_with_the_prover_else_the_node_count():
    assert awsb.stake_table_capacity(small_cfg(nodes=3)) == 10
    assert awsb.stake_table_capacity(small_cfg(nodes=50)) == 50
    assert awsb.stake_table_capacity(small_cfg(nodes=3, prover=True)) == 200


@pytest.mark.parametrize(("nodes", "capacity"), [(3, 10), (50, 50)])
def test_genesis_capacity_without_the_prover_is_unchanged(nodes, capacity):
    cfg = small_cfg(nodes=nodes)
    rendered = awsb.render_genesis(
        (HERE / "genesis.toml").read_bytes(),
        awsb.stake_table_capacity(cfg),
        "50mb",
        "0.6",
    ).decode()
    assert f"stake_table_capacity = {capacity}\n" in rendered
    assert re.search(rf"^capacity = {capacity}$", rendered, re.MULTILINE)


def test_capacity_200_reaches_genesis_and_the_ctl_env_with_the_prover():
    cfg = small_cfg(nodes=3, prover=True)
    rendered = awsb.render_genesis(
        (HERE / "genesis.toml").read_bytes(),
        awsb.stake_table_capacity(cfg),
        "50mb",
        "0.6",
    ).decode()
    assert "stake_table_capacity = 200\n" in rendered
    assert re.search(r"^capacity = 200$", rendered, re.MULTILINE)
    dotenv = awsb.parse_dotenv(DOTENV_TEXT + "ESPRESSO_STAKE_TABLE_CAPACITY=10\n")
    env = awsb.render_ctl_env(fleet(3), cfg, dotenv)
    assert "ESPRESSO_STAKE_TABLE_CAPACITY=200\n" in env
    plain = awsb.render_ctl_env(fleet(3), small_cfg(nodes=3), dotenv)
    assert "ESPRESSO_STAKE_TABLE_CAPACITY=10\n" in plain


def test_more_nodes_than_the_stake_table_capacity_refuse_the_prover():
    cap = awsb.PROVER_STAKE_TABLE_CAPACITY
    awsb.plan_hosts(small_cfg(nodes=cap, prover=True))
    with pytest.raises(awsb.Refused, match="--prover"):
        awsb.plan_hosts(small_cfg(nodes=cap + 1, prover=True))


def test_a_small_ctl_is_refused():
    awsb.check_prover_ctl(awsb.PROVER_MIN_MIB)
    with pytest.raises(awsb.Refused, match=str(awsb.PROVER_MIN_MIB)):
        awsb.check_prover_ctl(awsb.PROVER_MIN_MIB - 1)


def test_preflight_refuses_a_small_ctl(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr(awsb, "caller_account", lambda *_: "acct")
    monkeypatch.setattr(awsb, "tools_on_path", lambda *_: None)
    monkeypatch.setattr(
        awsb, "instance_specs", lambda *_: ("arm64", {"c8g.xlarge": 8192})
    )
    cfg = small_cfg(nodes=2, ctl_type="c8g.xlarge", prover=True)
    with pytest.raises(awsb.Refused, match="--prover"):
        awsb.preflight(FakeSystem(run=FakeRunner()), cfg, awsb.plan_hosts(cfg))


def test_a_fleet_with_a_small_ctl_refuses_the_prover(
    isolated: Path, monkeypatch: pytest.MonkeyPatch
):
    harness = FleetHarness(monkeypatch, isolated)
    runner = harness.up_fleet()
    memory = harness.fleet()["memory_mib"] | {awsb.CTL_TYPE: 8192}
    harness.set_fleet(memory_mib=memory)
    mark = len(runner.calls)
    with pytest.raises(awsb.Refused, match="--prover"):
        harness.run(runner, "--prover")
    assert len(runner.calls) == mark
    assert not harness.lock().exists()


def test_a_fleet_run_takes_the_prover_flag(
    isolated: Path, monkeypatch: pytest.MonkeyPatch
):
    harness = FleetHarness(monkeypatch, isolated)
    runner = harness.up_fleet()
    assert harness.run(runner, "--prover") == awsb.EXIT_OK
    run = netbench.read_json(harness.fleet_dir / "runs/01-colocated/manifest.json")
    assert run["config"]["prover"] is True
    assert run["config"]["epoch_height"] == awsb.SHORT_EPOCH_HEIGHT
    evidence: Any = {"clock_offset_ms": {}}
    meta = awsb.deployment_meta(run, evidence | {"prover": "prover-started"})
    assert (meta["prover"], meta["epoch_height"], meta["staking"]) == (
        "prover-started",
        awsb.SHORT_EPOCH_HEIGHT,
        True,
    )
    assert "prover" not in awsb.deployment_meta(run, evidence)


def test_a_fleet_without_the_prover_image_refuses_the_prover(
    isolated: Path, monkeypatch: pytest.MonkeyPatch
):
    harness = FleetHarness(monkeypatch, isolated)
    runner = harness.up_fleet()
    images = harness.fleet()["images"]
    del images["prover-service"]
    harness.set_fleet(images=images)
    with pytest.raises(awsb.Refused, match="prover-service"):
        harness.run(runner, "--prover")
    assert harness.run(runner) == awsb.EXIT_OK


def test_the_prover_image_is_pulled_on_ctl_only():
    assert "prover-service" in awsb.IMAGE_COMPONENTS
    assert "prover-service" in awsb.ROLE_IMAGES["ctl"]
    assert "prover-service" not in awsb.ROLE_IMAGES["query"]


def test_ctl_start_sh_creates_the_light_client_deploy_and_the_prover():
    script = awsb.render_start_sh(
        host("ctl", "ctl"), fake_images(), 32768, stake_nodes=5, prover=True
    )
    assert script == (START_SH_GOLDEN_DIR / "start-ctl-prover.sh").read_text()
    assert "--name deploy-lc" in script
    assert "--cpu-shares 2" in script
    assert "--cpus" not in script
    assert "RAYON_NUM_THREADS" not in script
    assert "-v /data/ark-srs:/root/.local/share/ark-srs" in script
    assert "unset ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS" in script
    assert "--use-mock" not in script


def test_ctl_start_sh_without_the_prover_has_no_prover_containers():
    script = awsb.render_start_sh(host("ctl", "ctl"), fake_images(), 32768)
    assert "deploy-lc" not in script
    assert "state-prover" not in script


def test_prover_script_runs_its_steps_in_order_and_traces_a_failure():
    script = awsb.render_prover_sh(LC_ADDRESS, "http://10.0.0.2:8080")
    at = [script.index(f"{step}\n") for step in awsb.PROVER_STEPS]
    assert at == sorted(at)
    assert "docker start deploy-lc" in script
    assert "http://10.0.0.2:8080/v1/node/block-height" in script
    assert LC_ADDRESS in script
    assert script.index("docker start state-prover") > script.index("eth_getCode")
    assert "trap" in script
    assert "failed $step" in script
    assert "/opt/bench/prover.log" in script


@pytest.mark.parametrize(
    ("log", "reached"),
    [
        ("", "not started"),
        ("1.5 node0-up\n", "node0-up"),
        ("1.5 node0-up\n2.5 lc-deployed\n", "lc-deployed"),
        ("1.5 node0-up\n2.5 failed lc-deployed\n", "failed lc-deployed"),
        (
            "1 node0-up\n2 lc-deployed\n3 lc-address-ok\n4 prover-started\n",
            "prover-started",
        ),
    ],
)
def test_prover_reached(log, reached):
    assert awsb.prover_reached(log) == reached


def test_prover_files_are_rendered_for_ctl_only_with_the_prover(tmp_path: Path):
    for name, cfg in (
        ("plain", small_cfg(epoch_height=300)),
        ("prover", small_cfg(epoch_height=300, prover=True)),
    ):
        run_dir = tmp_path / name
        hosts = awsb.plan_hosts(cfg)
        for spec in hosts:
            (run_dir / "hosts" / spec["name"]).mkdir(parents=True)
        manifest = {"hosts": hosts, "images": fake_images(), "memory_mib": MEMORY_MIB}
        awsb.render_host_files(run_dir, cfg, manifest, fleet(2))
    assert not (tmp_path / "plain/hosts/ctl/prover.sh").exists()
    script = (tmp_path / "prover/hosts/ctl/prover.sh").read_text()
    assert "http://10.0.0.2:8080/v1/node/block-height" in script
    assert "prover.sh" in awsb.SHIPPED_HOST_FILES


@pytest.mark.usefixtures("valid")
def test_the_prover_unit_starts_after_the_nodes_and_before_the_agent(
    run_harness: RunHarness,
):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner, "--prover") == awsb.EXIT_OK
    commands = [c[-1] for c in runner.calls if c[0] == "ssh"]
    node = max(i for i, c in enumerate(commands) if "docker start espresso-node" in c)
    prover = next(i for i, c in enumerate(commands) if "--unit bench-prover" in c)
    agent = next(i for i, c in enumerate(commands) if "--unit bench-agent" in c)
    assert node < prover < agent
    assert f"{awsb.BENCH_DIR}/prover.sh" in commands[prover]


@pytest.mark.usefixtures("valid")
def test_no_prover_unit_without_the_flag(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_OK
    assert not runner.ran("--unit bench-prover")
    assert not runner.ran("docker start deploy-lc")


def test_freeze_and_reset_stop_the_prover_unit(tmp_path: Path, isolated: Path):
    assert "state-prover" in awsb.FREEZE_CONTAINERS["ctl"]
    runner = Scripted({})
    awsb.freeze(remote(runner, isolated))
    ctl = next(c[-1] for c in runner.calls if c[0] == "ssh" and "anvil" in c[-1])
    assert "bench-prover" in ctl
    assert "bench-prover" in awsb.reset_script("ctl")
    assert "bench-prover" in awsb.reset_script("validator")


def test_manifests_from_before_the_prover_load_as_off():
    saved = awsb.config_to_json(small_cfg(prover=True))
    assert awsb.config_from_manifest(saved).prover is True
    del saved["prover"]
    assert awsb.config_from_manifest(saved).prover is False


def test_the_prover_is_in_the_config_hash_and_its_image_only_when_on():
    cfg = small_cfg(epoch_height=300)
    hosts = awsb.plan_hosts(cfg)
    genesis = b"genesis"
    without = {k: v for k, v in fake_images().items() if k != "prover-service"}
    changed = {
        **fake_images(),
        "prover-service": fake_image("ghcr.io/x/p:t") | {"digest": "sha256:1"},
    }

    def digest(cfg, images) -> str:
        return awsb.run_config_hash(cfg, hosts, images, genesis)

    on = dataclasses.replace(cfg, prover=True)
    assert digest(cfg, fake_images()) == digest(cfg, without)
    assert digest(cfg, fake_images()) == digest(cfg, changed)
    assert digest(on, fake_images()) != digest(cfg, fake_images())
    assert digest(on, fake_images()) != digest(on, changed)


def test_a_default_hash_ignores_both_new_images():
    cfg = small_cfg()
    hosts = awsb.plan_hosts(cfg)
    old = {
        k: v
        for k, v in fake_images().items()
        if k not in ("staking-cli", "prover-service")
    }
    assert awsb.run_config_hash(cfg, hosts, fake_images(), b"g") == (
        awsb.run_config_hash(cfg, hosts, old, b"g")
    )


def test_the_prover_adds_a_line_to_the_node_summary():
    assert not any("prover" in line for line in awsb.format_node_summary(small_cfg()))
    cfg = small_cfg(epoch_height=300, prover=True)
    assert any("prover" in line for line in awsb.format_node_summary(cfg))


def test_the_prover_log_reaches_the_evidence_and_the_deployment_meta(tmp_path: Path):
    manifest = aws_manifest()
    _, evidence = awsb.load_evidence(tmp_path, manifest, 0.0, 1.0)
    assert "prover" not in evidence
    manifest["config"]["prover"] = True
    _, evidence = awsb.load_evidence(tmp_path, manifest, 0.0, 1.0)
    assert evidence["prover"] == "not started"
    ctl = tmp_path / "hosts/ctl"
    ctl.mkdir(parents=True)
    (ctl / "prover.log").write_text("1 node0-up\n2 failed lc-deployed\n")
    _, evidence = awsb.load_evidence(tmp_path, manifest, 0.0, 1.0)
    assert evidence["prover"] == "failed lc-deployed"


def test_freeze_records_the_prover_state_before_stopping_the_containers(
    isolated: Path,
):
    runner = Scripted({})
    awsb.freeze(remote(runner, isolated), lc_address=LC_ADDRESS)
    ctl = next(c[-1] for c in runner.calls if c[0] == "ssh" and "anvil" in c[-1])
    assert ctl.index("prover-state.json") < ctl.index("docker stop")
    assert "docker inspect" in ctl
    assert "0x9fdb54a7" in ctl
    assert LC_ADDRESS in ctl
    runner = Scripted({})
    awsb.freeze(remote(runner, isolated))
    assert not any("prover-state.json" in c[-1] for c in runner.calls)


@pytest.mark.parametrize(
    ("log", "state", "text"),
    [
        ("1 node0-up\n", None, "node0-up"),
        ("1 node0-up\n2 failed lc-deployed\n", None, "failed lc-deployed"),
        ("1 prover-started\n", None, "prover-started"),
        (
            "1 prover-started\n",
            {"status": "running", "exit_code": 0, "lc_height": 25},
            "running, LC height 25",
        ),
        (
            "1 prover-started\n",
            {"status": "exited", "exit_code": 137, "lc_height": 0},
            "exited 137 at prover-started, LC height 0",
        ),
        (
            "1 prover-started\n",
            {"status": "running", "exit_code": 0, "lc_height": None},
            "running, LC height unavailable",
        ),
        (
            "1 node0-up\n2 failed lc-deployed\n",
            {"status": "created", "exit_code": 0, "lc_height": None},
            "failed lc-deployed",
        ),
    ],
)
def test_prover_summary(log, state, text):
    assert awsb.prover_summary(log, state) == text


def test_the_prover_state_file_reaches_the_evidence(tmp_path: Path):
    manifest = aws_manifest()
    manifest["config"]["prover"] = True
    ctl = tmp_path / "hosts/ctl"
    ctl.mkdir(parents=True)
    (ctl / "prover.log").write_text("1 prover-started\n")
    (ctl / "prover-state.json").write_text(
        '{"status": "running", "exit_code": 0, "lc_height": 7}'
    )
    _, evidence = awsb.load_evidence(tmp_path, manifest, 0.0, 1.0)
    assert evidence["prover"] == "running, LC height 7"


def test_the_fleet_prover_refusal_names_up_ctl_type():
    with pytest.raises(awsb.Refused, match=f"up --ctl-type {awsb.PROVER_CTL_TYPE}"):
        awsb.check_prover_ctl(awsb.PROVER_MIN_MIB - 1, fleet=True)
    with pytest.raises(awsb.Refused, match="pass --ctl-type") as plain:
        awsb.check_prover_ctl(awsb.PROVER_MIN_MIB - 1)
    assert "up --ctl-type" not in str(plain.value)


def test_staking_is_stored_explicitly_in_the_manifest_config():
    on = awsb.config_to_json(small_cfg(epoch_height=300))
    off = awsb.config_to_json(small_cfg())
    assert (on["staking"], off["staking"]) == (True, False)
    assert awsb.config_from_manifest(on).epoch_height == 300


def test_an_epoch_height_run_without_staking_is_not_labelled_staked(
    isolated: Path, monkeypatch: pytest.MonkeyPatch
):
    harness = FleetHarness(monkeypatch, isolated)
    runner = harness.up_fleet()
    assert harness.run(runner, "--epoch-height", "3000") == awsb.EXIT_OK
    run = netbench.read_json(harness.fleet_dir / "runs/01-colocated/manifest.json")
    evidence: Any = {"clock_offset_ms": {}}
    assert awsb.deployment_meta(run, evidence)["staking"] is True
    del run["config"]["staking"]
    meta = awsb.deployment_meta(run, evidence)
    assert "staking" not in meta
    assert meta["epoch_height"] == 3000


def test_the_prover_summary_says_the_load_driver_shares_ctl():
    cfg = small_cfg(epoch_height=300, prover=True)
    assert any(
        "load driver shares ctl" in line and "vCPUs" not in line
        for line in awsb.format_node_summary(cfg)
    )
