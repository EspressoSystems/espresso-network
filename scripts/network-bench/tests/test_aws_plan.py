import argparse
import dataclasses
import json
import re
import shutil
import subprocess
import threading
import urllib.error
import urllib.request
from collections.abc import Iterator
from dataclasses import replace
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import (
    C8IN_8XLARGE_PRICE_ITEM,
    DOTENV_TEXT,
    INSTANCE_PRICES,
    MEMORY_MIB,
    STS_CALL,
    FakeRegistry,
    FakeRunner,
    FakeSystem,
    awsb,
    completed,
    fake_image,
    fake_images,
    fake_preflight,
    fleet,
    isolated_env,
    plan_args,
    price_item,
    price_list,
    pricing,
    shot_estimate,
    sts_response,
)

HERE = Path(__file__).parents[1]
TF_DIR = Path("/bench-state/aws/run1/terraform")


def cmd_plan_exit(args: argparse.Namespace, run: FakeRunner) -> int:
    """Mirrors `main`'s single `Refused` catch."""
    try:
        return awsb.cmd_plan(args, FakeSystem(run=run))
    except awsb.Refused:
        return awsb.EXIT_REFUSED


def small_cfg(nodes: int = 2, submit: int = 1, **kw) -> Any:
    load = netbench.BenchConfig(submit_nodes=submit)
    return awsb.RunConfig(tag="x", nodes=nodes, load=load, **kw)


def parse_env(text: str) -> dict[str, str]:
    return dict(line.split("=", 1) for line in text.splitlines())


def host(name: str, role: str) -> dict:
    return {
        "name": name,
        "role": role,
        "instance_type": "x",
        "root_gb": 100,
        "root_iops": 3000,
        "root_mbps": 125,
    }


# REQ:awsbench-topology
def test_geometric_steps_end_at_the_target():
    steps = awsb.geometric_steps(4.0, 1.5, 200.0)
    assert steps[:4] == (4.0, 6.0, 9.0, 13.5)
    assert steps[-1] == 200.0
    assert steps[-2] < 200.0
    assert list(steps) == sorted(set(steps))


def test_peers_exclude_self_and_node0():
    for n in range(3, 9):
        for i in range(1, n):
            result = awsb.peers(i, n)
            assert i not in result
            assert 0 not in result
            assert len(result) == min(3, n - 2)


# REQ:awsbench-query-nodes
@pytest.mark.parametrize("n", range(7, 23))
@pytest.mark.parametrize("k", range(1, 5))
def test_peers_skip_query_nodes(n, k):
    for i in range(n):
        result = awsb.peers(i, n, k)
        assert i not in result
        assert len(result) == len(set(result))
        assert all(k <= j < n for j in result)
        assert len(result) == min(3, n - k - (i >= k))


def old_peers(i: int, n: int) -> list[int]:
    if i == 0:
        return list(range(1, min(3, n - 1) + 1))
    if n <= 2:
        return []
    count = min(3, n - 2)
    return [(i - 1 + k) % (n - 1) + 1 for k in range(1, count + 1)]


def test_peers_with_one_query_node_match_the_default():
    for n in range(2, 24):
        for i in range(n):
            assert awsb.peers(i, n, 1) == old_peers(i, n)
    assert awsb.peers(0, 22, 1) == [1, 2, 3]
    assert awsb.peers(5, 22, 1) == [6, 7, 8]
    assert awsb.peers(21, 22, 1) == [1, 2, 3]


def test_plan_hosts_gives_the_first_query_nodes_the_query_role():
    hosts = awsb.plan_hosts(small_cfg(nodes=8, query_nodes=4))
    roles = {h["name"]: h["role"] for h in hosts}
    assert [n for n, r in roles.items() if r == "query"] == [
        "node0",
        "node1",
        "node2",
        "node3",
    ]
    assert roles == {
        "ctl": "ctl",
        **{f"node{i}": "query" if i < 4 else "validator" for i in range(8)},
    }
    peers = awsb.plan_peers(hosts)
    assert all(
        p not in {"node0", "node1", "node2", "node3"}
        for i in range(4, 8)
        for p in peers[f"node{i}"]
    )


@pytest.mark.parametrize(
    "kw",
    [
        {"query_db": "volume"},
        {"query_db": "rds"},
        {"query_db": "tmpfs"},
        {"db_modes": ("colocated", "volume")},
    ],
)
def test_plan_hosts_refuses_several_query_nodes_off_colocated(kw):
    with pytest.raises(awsb.Refused, match="--query-db colocated"):
        awsb.plan_hosts(small_cfg(nodes=4, query_nodes=2, **kw))


@pytest.mark.parametrize("query_nodes", [0, 4])
def test_plan_hosts_refuses_query_nodes_outside_one_to_nodes_minus_one(query_nodes):
    with pytest.raises(awsb.Refused, match="--query-nodes"):
        awsb.plan_hosts(small_cfg(nodes=4, query_nodes=query_nodes))


def test_query_nodes_flag_reaches_the_config_and_the_hash():
    cfg = awsb.config_from_args(
        awsb.parse_args(["plan", "--tag", "x", "--query-nodes", "3"])
    )
    assert cfg.query_nodes == 3
    assert (
        awsb.config_from_args(awsb.parse_args(["plan", "--tag", "x"])).query_nodes == 1
    )
    hosts = awsb.plan_hosts(cfg)
    other = replace(cfg, query_nodes=2)
    assert awsb.run_config_hash(
        cfg, hosts, fake_images(), b"g"
    ) != awsb.run_config_hash(other, hosts, fake_images(), b"g")


def test_state_peers_of_a_validator_exclude_every_query_node():
    hosts = fleet(8, query_nodes=3)
    text = awsb.render_node_env(host("node5", "validator"), hosts, None)
    urls = parse_env(text)["ESPRESSO_NODE_STATE_PEERS"].split(",")
    queries = {awsb._node_url(hosts[f"node{j}"]) for j in range(3)}
    assert urls and not queries & set(urls)


def test_topology_marks_every_query_node():
    topo = awsb.topology(fleet(6, query_nodes=4), "sqlite")
    roles = topo["roles"]
    assert [n for n, r in roles.items() if "query" in r] == [
        f"node{i}" for i in range(4)
    ]
    assert roles["node3"] == "validator, query, sqlite"
    assert roles["node4"] == "validator"
    assert topo["query_node"] == "node0"


def test_support_plan_gates_postgres_on_every_query_host():
    plan = awsb.support_plan("colocated", [], "postgres", ["node0", "node1"])
    hosts = [s["host"] for s in plan if s["host"] != "ctl"]
    assert hosts == ["node0"] * 3 + ["node1"] * 3
    assert awsb.support_plan("colocated", [], "sqlite", ["node0", "node1"]) == (
        awsb.support_plan("colocated", [], "sqlite")
    )


def test_node_summary_prints_the_query_node_count():
    assert any(
        "4 query" in line
        for line in awsb.format_node_summary(small_cfg(8, query_nodes=4))
    )


# EDGE:old-manifest-no-chaos
def test_manifest_config_without_query_nodes_loads_as_one():
    saved = json.loads(json.dumps(awsb.config_to_json(small_cfg())))
    del saved["query_nodes"]
    assert awsb.config_from_manifest(saved).query_nodes == 1


# EDGE:awsbench-two-nodes
def test_two_nodes_one_validator_has_no_peers():
    assert awsb.peers(1, 2) == []
    hosts = awsb.plan_hosts(small_cfg())
    assert awsb.plan_peers(hosts) == {"node0": ["node1"], "node1": []}


def test_plan_hosts_refuses_a_single_node():
    with pytest.raises(awsb.Refused):
        awsb.plan_hosts(awsb.RunConfig(tag="x", nodes=1))


@pytest.mark.parametrize(
    ("steps", "validator_gb", "query_gb"),
    [
        (None, 100, 100),
        ((100.0, 200.0), 20 + 2 * 90, 20 + 3 * 90),
        ((1000.0,), 620, 920),
    ],
)
def test_node_root_gb_scales_with_offered_payload(steps, validator_gb, query_gb):
    load = (
        netbench.BenchConfig()
        if steps is None
        else netbench.BenchConfig(steps=steps, step_s=300)
    )
    cfg = awsb.RunConfig(tag="x", load=load)
    assert awsb.node_root_gb(cfg, query=False) == validator_gb
    assert awsb.node_root_gb(cfg, query=True) == query_gb


@pytest.mark.parametrize(
    ("keep_going", "tx_timeout_s", "expected"),
    [
        (False, 30, 60 + 3 * 30 + awsb.DRAIN_EXPECTED_S + 30),
        (True, 30, 60 + 2 * 30 + netbench.DRAIN_MAX_S + netbench.DRAIN_GRACE_S + 30),
        (True, 600, 60 + 2 * 30 + 600 + netbench.DRAIN_GRACE_S + 600),
    ],
)
def test_load_seconds(keep_going, tx_timeout_s, expected):
    load = netbench.BenchConfig(
        steps=(4.0, 8.0),
        step_s=30,
        warmup_s=60,
        tx_timeout_s=tx_timeout_s,
        keep_going=keep_going,
    )
    assert awsb.load_seconds(load) == expected


@pytest.mark.parametrize(
    ("flags", "keep_going"), [((), False), (("--keep-going",), True)]
)
def test_keep_going_flag(flags, keep_going):
    args = awsb.parse_args(["run", "--tag", "x", *flags])
    assert awsb.config_from_args(args).load.keep_going == keep_going


def test_worst_uses_ready_timeout_and_collect_max():
    expected_s, worst_s = awsb.shot_seconds(awsb.RunConfig(tag="x"))
    assert worst_s - expected_s == (
        awsb.READY_TIMEOUT_S
        - awsb.READY_EXPECTED_S
        + awsb.COLLECT_MAX_S
        - awsb.COLLECT_EXPECTED_S
        + netbench.DRAIN_MAX_S
        + netbench.DRAIN_GRACE_S
        - awsb.DRAIN_EXPECTED_S
    )


# REQ:fleet-cost
def test_a_run_has_no_provision_or_destroy():
    cfg = awsb.RunConfig(tag="x")
    run_expected, run_worst = awsb.run_seconds(cfg)
    shot_expected, shot_worst = awsb.shot_seconds(cfg)
    fixed = awsb.PROVISION_S + awsb.DESTROY_S
    assert run_expected - awsb.RESET_EXPECTED_S == shot_expected - fixed
    assert run_worst - awsb.RESET_MAX_S == shot_worst - fixed


# REQ:awsbench-cost-bound
def test_estimate_matches_hand_computed_totals():
    # Literal dollar values for this 2-node config at PRICES, hand-computed independently
    # of cost_estimate's formula, so a formula regression trips this test.
    cfg = small_cfg(pg_iops=6000, pg_mbps=500)
    estimate = shot_estimate(awsb.plan_hosts(cfg), cfg)
    assert estimate["expected_s"] == 1670.0
    assert estimate["ttl_s"] == 3580.0
    assert estimate["expected_usd"] == pytest.approx(0.865536, abs=1e-5)
    assert estimate["bound_usd"] == pytest.approx(1.928954, abs=1e-5)


def test_bound_is_cost_at_ttl():
    cfg = small_cfg(pg_iops=6000, pg_mbps=500)
    estimate = shot_estimate(awsb.plan_hosts(cfg), cfg)
    ratio = (estimate["ttl_s"] + awsb.BOOT_ALLOWANCE_S) / estimate["expected_s"]
    # Every line scales with the duration except egress, which is flat per host.
    egress = next(line["usd"] for line in estimate["lines"] if line["item"] == "egress")
    scaled = (estimate["expected_usd"] - egress) * ratio + egress
    assert estimate["bound_usd"] == pytest.approx(scaled, abs=1e-4)


def test_estimate_summary_has_expected_bound_and_limit():
    cfg = small_cfg()
    estimate = shot_estimate(awsb.plan_hosts(cfg), cfg)
    text = awsb.format_estimate_summary(estimate, cfg.max_usd)
    assert f"expected ${estimate['expected_usd']:.2f}" in text
    assert f"hard bound ${estimate['bound_usd']:.2f}" in text
    assert "limit $10.00" in text


def test_run_estimate_is_the_fleet_rate_over_the_phases():
    cfg = small_cfg()
    manifest = {
        "estimate": shot_estimate(awsb.plan_hosts(cfg), cfg),
        "config": {"tag": "x"},
    }
    estimate = awsb.estimate_run(manifest, cfg)
    assert estimate["worst_s"] == awsb.run_seconds(cfg)[1]
    rate = awsb.estimate_rate(manifest["estimate"])
    assert estimate["worst_usd"] == pytest.approx(rate * estimate["worst_s"] / 3600)
    assert estimate["expected_usd"] < estimate["worst_usd"]


@pytest.mark.parametrize(("yes", "prompts"), [(True, 0), (False, 1)])
def test_a_confirmed_create_goes_ahead(tmp_path, yes, prompts):
    system = FakeSystem(answer=True)
    awsb.confirm_create(system.ask, awsb.RunConfig(tag="x", yes=yes), tmp_path, 2)
    assert len(system.prompts) == prompts


def test_an_unanswered_prompt_refuses(tmp_path):
    system = FakeSystem(answer=False)
    with pytest.raises(awsb.Refused, match="not confirmed"):
        awsb.confirm_create(system.ask, awsb.RunConfig(tag="x"), tmp_path, 2)
    assert len(system.prompts) == 1


@pytest.fixture
def fleet_dir(isolated: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    return isolated_env(monkeypatch, isolated, "run1") / "run1"


@pytest.fixture
def preflighted(monkeypatch: pytest.MonkeyPatch) -> list[tuple]:
    calls: list[tuple] = []

    def preflight(*args: object) -> dict:
        calls.append(args)
        return fake_preflight()

    monkeypatch.setattr(awsb, "preflight", preflight)
    return calls


# REQ:awsbench-render
@pytest.mark.usefixtures("preflighted")
def test_plan_renders_manifest_and_peers(fleet_dir):
    code = awsb.cmd_plan(plan_args(), FakeSystem(run=FakeRunner(states=[])))
    assert code == awsb.EXIT_OK
    for name in (
        "runs/01-run/genesis.toml",
        "runs/01-run/config.json",
        "hosts/ctl/user-data.sh",
        "hosts/node0/user-data.sh",
        "terraform/main.tf",
        "driver.log",
        "events.jsonl",
    ):
        assert (fleet_dir / name).exists(), name
    manifest = netbench.read_json(fleet_dir / "fleet.json")
    assert manifest["phase"] == "planned"
    assert manifest["git_rev"] == "a" * 40
    assert manifest["peers"] == {"node0": ["node1"], "node1": []}
    assert len(manifest["hosts"]) == 3
    assert "expires_at" not in manifest
    assert not (fleet_dir / "manifest.json").exists()
    assert not (fleet_dir / "diff.patch").exists()
    run_manifest = netbench.read_json(fleet_dir / "runs/01-run/manifest.json")
    assert run_manifest["fleet"] == "run1"
    assert run_manifest["hosts"] == manifest["hosts"]
    tfvars = netbench.read_json(fleet_dir / "terraform" / "terraform.tfvars.json")
    assert tfvars["expires_at"] == ""
    assert tfvars["region"] == "eu-west-1"
    assert not {"account_id", "profile"} & tfvars.keys()


@pytest.mark.usefixtures("preflighted")
def test_plan_in_another_region_records_it_in_tfvars_and_manifests(fleet_dir):
    args = plan_args("--region", "us-east-1")
    assert awsb.cmd_plan(args, FakeSystem(run=FakeRunner(states=[]))) == awsb.EXIT_OK
    tfvars = netbench.read_json(fleet_dir / "terraform" / "terraform.tfvars.json")
    assert tfvars["region"] == "us-east-1"
    fleet = netbench.read_json(fleet_dir / "fleet.json")
    run_manifest = netbench.read_json(fleet_dir / "runs/01-run/manifest.json")
    assert fleet["config"]["region"] == run_manifest["config"]["region"] == "us-east-1"
    assert awsb.manifest_region(fleet) == "us-east-1"


def test_config_from_a_manifest_without_a_region_is_eu_west_1():
    saved = awsb.config_to_json(small_cfg())
    del saved["region"]
    assert awsb.config_from_manifest(saved).region == awsb.LEGACY_REGION == "eu-west-1"


@pytest.mark.usefixtures("preflighted")
def test_key_is_generated_into_the_fleet_dir(fleet_dir):
    runner = FakeRunner(states=[])
    assert awsb.cmd_plan(plan_args(), FakeSystem(run=runner)) == awsb.EXIT_OK
    key = fleet_dir / "ssh" / "id_ed25519"
    keygen = next(c for c in runner.calls if c[0] == "ssh-keygen")
    assert keygen == [
        "ssh-keygen",
        "-q",
        "-t",
        "ed25519",
        "-N",
        "",
        "-C",
        "run1",
        "-f",
        str(awsb.OUT_ROOT / "run1" / "ssh" / "id_ed25519"),
    ]
    assert key.stat().st_mode & 0o777 == 0o600
    tfvars = netbench.read_json(fleet_dir / "terraform" / "terraform.tfvars.json")
    assert tfvars["ssh_public_key"] == "ssh-ed25519 GENERATED run"
    manifest = netbench.read_json(fleet_dir / "fleet.json")
    assert manifest["ssh_public_key"] == "ssh-ed25519 GENERATED run"


def test_preflight_requires_ssh_keygen():
    system = FakeSystem(tools={"tofu", "aws", "ssh", "scp", "rsync", "git"})
    with pytest.raises(awsb.Refused, match="ssh-keygen"):
        awsb.preflight(system, awsb.RunConfig(tag="x"), [])


@pytest.mark.usefixtures("preflighted")
def test_plan_logs_preflight_before_cost(fleet_dir):
    awsb.cmd_plan(plan_args(), FakeSystem(run=FakeRunner(states=[])))
    lines = (fleet_dir / "driver.log").read_text().splitlines()
    account = next(i for i, line in enumerate(lines) if "account 0275" in line)
    cost = next(i for i, line in enumerate(lines) if "expected $" in line)
    assert account < cost


# REQ:awsbench-budget-refusal
@pytest.mark.usefixtures("fleet_dir", "preflighted")
def test_budget_refusal_makes_no_tofu_call():
    runner = FakeRunner(states=[])
    code = cmd_plan_exit(plan_args("--max-usd", "0.01", nodes="5"), run=runner)
    assert code == awsb.EXIT_REFUSED
    assert not runner.ran("tofu")


# EDGE:awsbench-name-collision
@pytest.mark.usefixtures("fleet_dir")
def test_name_collision_refuses_before_any_aws_call(preflighted):
    first = awsb.cmd_plan(plan_args(), FakeSystem(run=FakeRunner(states=[])))
    assert first == awsb.EXIT_OK
    runner = FakeRunner({})
    assert cmd_plan_exit(plan_args(), run=runner) == awsb.EXIT_REFUSED
    assert runner.calls == []


# REQ:awsbench-account-guard
@pytest.mark.usefixtures("fleet_dir")
def test_account_mismatch_refuses_after_one_call():
    runner = FakeRunner({STS_CALL: sts_response("999999999999")})
    assert cmd_plan_exit(plan_args(), run=runner) == awsb.EXIT_REFUSED
    assert len(runner.calls) == 1


# REQ:awsbench-account-guard
def test_caller_account_returns_the_matching_account():
    runner = FakeRunner({STS_CALL: sts_response(awsb.ACCOUNT)})
    assert awsb.caller_account(runner, awsb.REGION) == awsb.ACCOUNT


def offerings(*azs: str) -> subprocess.CompletedProcess:
    body = {"InstanceTypeOfferings": [{"Location": az} for az in azs]}
    return completed(stdout=json.dumps(body))


def test_capable_az_is_the_lowest_az_offering_every_type():
    runner = FakeRunner()
    runner.respond(
        "Values=c8g.2xlarge", lambda _: offerings("eu-west-1c", "eu-west-1b")
    )
    runner.respond(
        "Values=c8g.4xlarge",
        lambda _: offerings("eu-west-1b", "eu-west-1c", "eu-west-1a"),
    )
    assert (
        awsb.capable_az(runner, awsb.REGION, {"c8g.2xlarge", "c8g.4xlarge"})
        == "eu-west-1b"
    )


def test_no_capable_az_refuses():
    runner = FakeRunner()
    runner.respond("Values=c8g.2xlarge", lambda _: offerings("eu-west-1a"))
    runner.respond("Values=c8g.4xlarge", lambda _: offerings("eu-west-1b"))
    with pytest.raises(awsb.Refused, match="no AZ in eu-west-1 offers"):
        awsb.capable_az(runner, awsb.REGION, {"c8g.2xlarge", "c8g.4xlarge"})


def test_plan_runs_preflight_and_tofu_plan_without_apply(fleet_dir, preflighted):
    runner = FakeRunner(states=[])
    assert awsb.cmd_plan(plan_args(), FakeSystem(run=runner)) == awsb.EXIT_OK
    assert len(preflighted) == 1
    manifest = netbench.read_json(fleet_dir / "fleet.json")
    assert manifest["phase"] == "planned"
    assert manifest["az"] == "eu-west-1b"
    assert manifest["ami_id"] == "ami-0abc"
    assert set(manifest["images"]) == set(fake_images())
    assert (fleet_dir / "terraform/plan.txt").read_text() == "plan"
    assert runner.ran("tofu", "plan")
    assert not runner.ran("tofu", "apply")


def test_preflight_resolves_az_ami_and_images(monkeypatch: pytest.MonkeyPatch):
    cfg = small_cfg()
    prefix = ("aws", "--profile", "timeboost-dev")
    offerings = {"InstanceTypeOfferings": [{"Location": "eu-west-1a"}]}
    runner = FakeRunner(
        {
            STS_CALL: sts_response(awsb.ACCOUNT),
            (*prefix, "ec2", "describe-instance-type-offerings"): completed(
                stdout=json.dumps(offerings)
            ),
            (*prefix, "ec2", "describe-images"): completed(
                stdout=json.dumps("ami-0abc")
            ),
        }
    )
    runner.respond(
        "describe-instance-types",
        lambda _: instance_types({"c8g.2xlarge": "arm64", "c8g.4xlarge": "arm64"}),
    )
    runner.respond("get-products", pricing)
    monkeypatch.setattr(
        awsb, "resolve_image", lambda _http_get, ref, _platform: fake_image(ref)
    )
    result = awsb.preflight(FakeSystem(run=runner), cfg, awsb.plan_hosts(cfg))
    assert result["arch"] == "arm64"
    assert result["instance_prices"] == {
        "c8g.2xlarge": 0.34112,
        "c8g.4xlarge": 0.68224,
    }
    assert result["memory_mib"] == {"c8g.2xlarge": 16384, "c8g.4xlarge": 32768}
    assert result["az"] == "eu-west-1a"
    assert result["ami_id"] == "ami-0abc"
    assert set(result["images"]) == set(awsb.image_refs(cfg))


LOCK_ERROR = """\
\x1b[31m╷\x1b[0m\x1b[0m
\x1b[31m│\x1b[0m \x1b[0m\x1b[1m\x1b[31mError: \x1b[0m\x1b[0m\x1b[1mError acquiring the state lock\x1b[0m
\x1b[31m│\x1b[0m \x1b[0m
\x1b[31m│\x1b[0m \x1b[0m\x1b[0mError message: resource temporarily unavailable
\x1b[31m│\x1b[0m \x1b[0mLock Info:
\x1b[31m│\x1b[0m \x1b[0m  ID:        82827aee-aaa4-486e-a5d6-8bb5751a086e
\x1b[31m╵\x1b[0m\x1b[0m
"""


@pytest.mark.parametrize(
    ("stage", "stderr", "message"),
    [
        (
            "apply",
            "Error: creating\nVcpuLimitExceeded: x\n\nlater\n",
            "tofu apply failed: Error: creating; VcpuLimitExceeded: x",
        ),
        (
            "apply",
            "Error: one\nError: two\n",
            "tofu apply failed: Error: one",
        ),
        ("destroy", "boom\n\x1b[31mlast\x1b[0m\n\n", "tofu destroy failed: last"),
        ("destroy", " \n\n", "tofu destroy failed: exit 1"),
        (
            "destroy",
            "noise\n" + LOCK_ERROR + "tail\n",
            (
                "tofu destroy failed: Error: Error acquiring the state lock; "
                "Error message: resource temporarily unavailable; Lock Info:; "
                "ID:        82827aee-aaa4-486e-a5d6-8bb5751a086e"
            ),
        ),
    ],
    ids=[
        "plain-error",
        "next-error",
        "no-error-block",
        "blank-stderr",
        "boxed-lock-error",
    ],
)
def test_tofu_failure_raises_with_stage_and_error_block(stage, stderr, message):
    runner = FakeRunner(
        {("tofu", f"-chdir={TF_DIR}", stage): completed(returncode=1, stderr=stderr)}
    )
    tf = awsb.Terraform(runner, TF_DIR, env={})
    with pytest.raises(awsb.TfFailed) as err:
        getattr(tf, stage)()
    assert err.value.stage == stage
    assert str(err.value) == message


def resolve_with(registry: FakeRegistry, platform: str = "linux/arm64") -> dict:
    return awsb.resolve_image(
        FakeSystem(http=registry).http_get, registry.ref, platform
    )


# REQ:awsbench-image-check
def test_resolve_image_digest_platforms_and_revision():
    server = FakeRegistry(
        repository="test/image",
        tag="v1",
        platforms=[("linux", "amd64"), ("linux", "arm64")],
        revision="abc1234",
    )
    info = resolve_with(server)
    assert info["digest"] == server.digests[("linux", "arm64")]
    assert sorted(info["platforms"]) == ["linux/amd64", "linux/arm64"]
    assert info["revision"] == "abc1234"
    assert info["ref"] == server.ref


def test_resolve_image_single_manifest_without_index(registry):
    registry.routes[f"{registry.manifest_prefix}v1"] = registry.single
    info = resolve_with(registry)
    assert info["platforms"] == ["linux/arm64"]
    assert info["digest"] == registry.single_digest


def test_resolve_image_ignores_attestation_platform():
    # buildx publishes an extra unknown/unknown manifest for SBOM/provenance attestations.
    server = FakeRegistry("x", "v1", [("unknown", "unknown"), ("linux", "arm64")])
    assert resolve_with(server)["platforms"] == ["linux/arm64"]


def no_arm64() -> FakeRegistry:
    return FakeRegistry("x", "v1", [("linux", "amd64")])


def missing_tag() -> FakeRegistry:
    return FakeRegistry("x", "missing", [("linux", "arm64")])


def denied_token() -> FakeRegistry:
    server = FakeRegistry("x", "v1", [("linux", "arm64")])
    server.routes["/token"] = (401, {}, b"")
    return server


def forbidden_manifest() -> FakeRegistry:
    server = FakeRegistry("x", "v1", [("linux", "arm64")])
    server.routes[f"{server.manifest_prefix}v1"] = (403, {}, b"")
    return server


# EDGE:awsbench-image-no-arm64 / EDGE:awsbench-image-private
@pytest.mark.parametrize(
    ("make", "message"),
    [
        (no_arm64, "no linux/arm64 platform"),
        (missing_tag, "image not found"),
        (denied_token, "token request"),
        (forbidden_manifest, "private or inaccessible"),
    ],
)
def test_resolve_image_refuses(make, message):
    server = make()
    with pytest.raises(awsb.Refused) as err:
        resolve_with(server)
    assert message in str(err.value)
    assert server.ref in str(err.value)


def test_resolve_image_picks_the_wanted_platform():
    server = FakeRegistry("x", "v1", [("linux", "arm64"), ("linux", "amd64")])
    info = resolve_with(server, "linux/amd64")
    assert info["digest"] == server.digests[("linux", "amd64")]


def test_resolve_image_refusal_names_the_wanted_platform():
    server = FakeRegistry("x", "v1", [("linux", "arm64")])
    with pytest.raises(awsb.Refused, match="no linux/amd64 platform .* linux/arm64"):
        resolve_with(server, "linux/amd64")


def instance_types(archs: dict[str, str | list[str]]) -> subprocess.CompletedProcess:
    """`describe-instance-types` for type -> EC2 architecture name(s), with the type's real
    memory."""
    body = {
        "InstanceTypes": [
            {
                "InstanceType": t,
                "ProcessorInfo": {
                    "SupportedArchitectures": [a] if isinstance(a, str) else a
                },
                "MemoryInfo": {"SizeInMiB": MEMORY_MIB[t]},
            }
            for t, a in archs.items()
        ]
    }
    return completed(stdout=json.dumps(body))


@pytest.mark.parametrize(
    ("supported", "arch"),
    [(["arm64"], "arm64"), (["x86_64"], "amd64"), (["i386", "x86_64"], "amd64")],
)
def test_instance_specs_maps_ec2_architectures(supported: list[str], arch: str):
    types = {"c8i.2xlarge", "c8i.8xlarge"}
    runner = FakeRunner()
    runner.respond(
        "describe-instance-types",
        lambda _: instance_types(dict.fromkeys(types, supported)),
    )
    assert awsb.instance_specs(runner, awsb.REGION, types) == (
        arch,
        {"c8i.2xlarge": 16384, "c8i.8xlarge": 65536},
    )


def test_instance_specs_refuses_a_type_without_memory():
    body = {
        "InstanceTypes": [
            {
                "InstanceType": "c8g.4xlarge",
                "ProcessorInfo": {"SupportedArchitectures": ["arm64"]},
            }
        ]
    }
    runner = FakeRunner()
    runner.respond(
        "describe-instance-types", lambda _: completed(stdout=json.dumps(body))
    )
    with pytest.raises(KeyError, match="MemoryInfo"):
        awsb.instance_specs(runner, awsb.REGION, {"c8g.4xlarge"})


def intel_preflight_runner(ctl_arch: str = "x86_64") -> FakeRunner:
    runner = FakeRunner(
        {
            STS_CALL: sts_response(awsb.ACCOUNT),
            ("aws", "--profile", "timeboost-dev", "ec2", "describe-images"): completed(
                stdout=json.dumps("ami-0amd")
            ),
        }
    )
    runner.respond(
        "describe-instance-types",
        lambda _: instance_types({"c8i.2xlarge": ctl_arch, "c8i.4xlarge": "x86_64"}),
    )
    runner.respond(
        "describe-instance-type-offerings", lambda _: offerings("eu-west-1a")
    )
    runner.respond("get-products", pricing)
    return runner


INTEL = {"node_type": "c8i.4xlarge", "ctl_type": "c8i.2xlarge"}


def intel_cfg() -> Any:
    return small_cfg(node_type=INTEL["node_type"], ctl_type=INTEL["ctl_type"])


def test_preflight_on_intel_resolves_the_amd64_ami_and_images(
    monkeypatch: pytest.MonkeyPatch,
):
    cfg = intel_cfg()
    runner = intel_preflight_runner()
    platforms: list[str] = []

    def resolve(_http_get, ref: str, platform: str) -> dict:
        platforms.append(platform)
        return fake_image(ref)

    monkeypatch.setattr(awsb, "resolve_image", resolve)
    result = awsb.preflight(FakeSystem(run=runner), cfg, awsb.plan_hosts(cfg))
    assert result["arch"] == "amd64"
    assert result["ami_id"] == "ami-0amd"
    assert runner.ran("describe-instance-types", "c8i.2xlarge", "c8i.4xlarge")
    assert runner.ran("describe-images", "ubuntu-noble-24.04-amd64-server-*")
    assert set(platforms) == {"linux/amd64"}


def test_preflight_refuses_node_and_ctl_types_of_different_architectures():
    cfg = intel_cfg()
    runner = intel_preflight_runner(ctl_arch="arm64")
    with pytest.raises(awsb.Refused, match="c8i.2xlarge arm64, c8i.4xlarge amd64"):
        awsb.preflight(FakeSystem(run=runner), cfg, awsb.plan_hosts(cfg))
    assert not runner.ran("describe-images")


@pytest.mark.usefixtures("preflighted")
def test_node_and_ctl_type_flags_reach_the_hosts_and_manifest(
    fleet_dir, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "preflight", lambda *_: fake_preflight("amd64"))
    args = plan_args("--node-type", "c8i.4xlarge", "--ctl-type", "c8i.2xlarge")
    assert awsb.cmd_plan(args, FakeSystem(run=FakeRunner(states=[]))) == awsb.EXIT_OK
    manifest = netbench.read_json(fleet_dir / "runs/01-run/manifest.json")
    types = {h["name"]: h["instance_type"] for h in manifest["hosts"]}
    assert types == {
        "ctl": "c8i.2xlarge",
        "node0": "c8i.4xlarge",
        "node1": "c8i.4xlarge",
    }
    assert manifest["config"] | INTEL == manifest["config"]
    assert manifest["arch"] == "amd64"
    assert manifest["memory_mib"] == MEMORY_MIB
    assert {line["item"] for line in manifest["estimate"]["lines"]} >= {
        "instance c8i.2xlarge",
        "instance c8i.4xlarge",
    }


@pytest.mark.parametrize("key", ["node_type", "ctl_type"])
def test_config_from_a_manifest_without_an_instance_type_raises(key: str):
    saved = awsb.config_to_json(small_cfg())
    del saved[key]
    with pytest.raises(KeyError, match=key):
        awsb.config_from_manifest(saved)


def pricing_runner() -> FakeRunner:
    runner = FakeRunner()
    runner.respond("get-products", pricing)
    return runner


def test_instance_price_reads_the_real_price_list_shape():
    runner = FakeRunner()
    runner.respond("get-products", lambda _: price_list(C8IN_8XLARGE_PRICE_ITEM))
    assert awsb.instance_price(runner, "eu-central-1", "c8in.8xlarge") == 2.45952
    (call,) = runner.calls
    assert call[call.index("--region") + 1] == "us-east-1"
    assert call[call.index("--service-code") + 1] == "AmazonEC2"
    for field, value in {
        "instanceType": "c8in.8xlarge",
        "regionCode": "eu-central-1",
        "operatingSystem": "Linux",
        "tenancy": "Shared",
        "preInstalledSw": "NA",
        "capacitystatus": "Used",
    }.items():
        assert f"Type=TERM_MATCH,Field={field},Value={value}" in call


def test_instance_price_of_an_unknown_type_raises():
    with pytest.raises(awsb.Refused, match=r"m7a\.large in eu-west-1: 0 on-demand"):
        awsb.instance_price(pricing_runner(), awsb.REGION, "m7a.large")


def test_instance_price_of_several_products_raises():
    item = price_item("c8g.4xlarge", 0.68224)
    runner = FakeRunner()
    runner.respond("get-products", lambda _: price_list(item, item))
    with pytest.raises(awsb.Refused, match=r"c8g\.4xlarge in eu-west-1: 2 on-demand"):
        awsb.instance_price(runner, awsb.REGION, "c8g.4xlarge")


@pytest.mark.usefixtures("fleet_dir")
def test_plan_of_an_unpriced_type_fails_in_preflight(monkeypatch: pytest.MonkeyPatch):
    runner = intel_preflight_runner()
    args = plan_args("--node-type", "m7a.large", "--ctl-type", "m7a.large")
    with pytest.raises(awsb.Refused, match=r"m7a\.large in eu-west-1: 0 on-demand"):
        awsb.cmd_plan(args, FakeSystem(run=runner))
    assert not runner.ran("describe-images")


@pytest.fixture
def served(registry: FakeRegistry, monkeypatch: pytest.MonkeyPatch) -> Iterator[str]:
    """Loopback server relaying to a `FakeRegistry`: the boundary test for `_http_get`."""
    for proxy in ("http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY"):
        monkeypatch.delenv(proxy, raising=False)
    monkeypatch.setenv("no_proxy", "127.0.0.1")

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, format, *args):
            pass

        def do_GET(self):
            status, headers, body = registry(
                f"https://{registry.host}{self.path}", dict(self.headers)
            )
            self.send_response(status)
            for name, value in headers.items():
                self.send_header(name, value)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    server = HTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(
        target=server.serve_forever, kwargs={"poll_interval": 0.01}, daemon=True
    )
    thread.start()
    yield f"http://127.0.0.1:{server.server_address[1]}"
    server.shutdown()
    thread.join()
    server.server_close()


@pytest.mark.slow
def test_http_get_returns_body(served, registry):
    url = f"{served}/v2/test/image/blobs/{registry.config_digest}"
    status, _, body = awsb._http_get(url, {})
    assert status == 200
    assert json.loads(body)["architecture"] == "arm64"


@pytest.mark.slow
def test_http_get_retries_a_failed_connection(served, registry, monkeypatch):
    urlopen = urllib.request.urlopen
    calls = []

    def flaky(request, timeout):
        calls.append(request.full_url)
        if len(calls) == 1:
            raise urllib.error.URLError(TimeoutError("timed out"))
        return urlopen(request, timeout=timeout)

    monkeypatch.setattr(urllib.request, "urlopen", flaky)
    url = f"{served}/v2/test/image/blobs/{registry.config_digest}"
    status, _, _ = awsb._http_get(url, {})
    assert (status, len(calls)) == (200, 2)


def test_http_get_gives_up_after_its_attempts(monkeypatch):
    calls = []

    def down(request, timeout):
        calls.append(request.full_url)
        raise urllib.error.URLError(TimeoutError("timed out"))

    monkeypatch.setattr(urllib.request, "urlopen", down)
    with pytest.raises(urllib.error.URLError):
        awsb._http_get("https://registry.invalid/v2/", {})
    assert len(calls) == awsb.REGISTRY_GET_ATTEMPTS


@pytest.mark.slow
def test_http_get_returns_error_status_not_raised(served):
    status, headers, _ = awsb._http_get(f"{served}/v2/test/image/manifests/v1", {})
    assert status == 401
    assert "realm=" in headers["WWW-Authenticate"]


@pytest.mark.parametrize(
    ("ref", "parsed"),
    [
        (
            "ghcr.io/espressosystems/espresso-network/espresso-node:x",
            ("ghcr.io", "espressosystems/espresso-network/espresso-node", "x"),
        ),
        # https://docker.io/v2/... 302s to www.docker.com.
        (
            "docker.io/library/postgres:16",
            ("registry-1.docker.io", "library/postgres", "16"),
        ),
    ],
)
def test_parse_ref(ref, parsed):
    assert awsb.parse_ref(ref) == parsed


TOFU = shutil.which("tofu")


@pytest.mark.slow
@pytest.mark.skipif(TOFU is None, reason="tofu/opentofu not on PATH")
def test_tofu_validate(tmp_path):
    """TEST:awsbench-tofu-validate-ok: `init` needs the network for the providers."""
    assert TOFU is not None
    module = tmp_path / "terraform"
    shutil.copytree(HERE / "aws" / "terraform", module)
    for args in (["init", "-input=false", "-backend=false"], ["validate"]):
        result = subprocess.run(
            [TOFU, f"-chdir={module}", *args],
            capture_output=True,
            text=True,
            check=False,
        )
        assert result.returncode == 0, result.stderr


@pytest.mark.parametrize(("n", "capacity"), [(3, 10), (50, 50)])
def test_genesis_capacity_at_least_ten(n, capacity):
    rendered = awsb.render_genesis(
        (HERE / "genesis.toml").read_bytes(), n=n, max_block_size="50mb"
    ).decode()
    assert f"stake_table_capacity = {capacity}" in rendered
    assert re.search(rf"^capacity = {capacity}$", rendered, re.MULTILINE)


def test_genesis_max_block_size_replaces_both_chain_configs():
    template = (HERE / "genesis.toml").read_bytes()
    rendered = awsb.render_genesis(template, n=5, max_block_size="30mb").decode()
    assert (
        re.findall(r"^max_block_size = (.*)$", rendered, re.MULTILINE) == ['"30mb"'] * 2
    )
    assert "100mb" not in rendered
    with pytest.raises(ValueError, match="2 max_block_size"):
        awsb.render_genesis(b"capacity = 1\nstake_table_capacity = 1\n", 5, "30mb")


@pytest.mark.parametrize("bad", ["50", "mb", "50 mb", "5.5mb", "50tb", "", "0kb"])
def test_max_block_size_flag_rejects_bad_formats(bad):
    with pytest.raises(SystemExit):
        node_env_config("--max-block-size", bad)


def test_max_block_size_flag_defaults_and_differs_in_the_config_hash():
    assert node_env_config().max_block_size == "50mb"
    assert node_env_config("--max-block-size", "100MB").max_block_size == "100MB"
    template = (HERE / "genesis.toml").read_bytes()
    cfg = small_cfg()
    hosts = awsb.plan_hosts(cfg)
    hashes = {
        awsb.run_config_hash(
            cfg, hosts, {}, awsb.render_genesis(template, cfg.nodes, size)
        )
        for size in ("30mb", "50mb")
    }
    assert len(hashes) == 2


@pytest.mark.usefixtures("preflighted")
def test_plan_writes_the_max_block_size_into_the_run_genesis(fleet_dir):
    args = plan_args()
    args.max_block_size = "30mb"
    awsb.cmd_plan(args, FakeSystem(run=FakeRunner(states=[])))
    genesis = (fleet_dir / "runs/01-run/genesis.toml").read_text()
    assert genesis.count('max_block_size = "30mb"') == 2


def test_genesis_raises_if_template_shape_changes():
    with pytest.raises(ValueError):
        awsb.render_genesis(b"no capacity fields here", n=5, max_block_size="50mb")


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        (
            '# comment\nFOO=bar\nQUOTED="a b c"\n\nINTERVAL=1m\n',
            {"FOO": "bar", "QUOTED": "a b c", "INTERVAL": "1m"},
        ),
        ("FOO=bar\nBAZ=${FOO}/baz\n", {"FOO": "bar", "BAZ": "bar/baz"}),
    ],
)
def test_parse_dotenv(text, expected):
    assert awsb.parse_dotenv(text) == expected


@pytest.mark.parametrize(
    ("text", "error"),
    [("BAZ=${UNKNOWN}\n", KeyError), ("FOO=bar\nnot a line\n", ValueError)],
)
def test_parse_dotenv_errors(text, error):
    with pytest.raises(error):
        awsb.parse_dotenv(text)


@pytest.mark.slow
def test_real_env_file_parses():
    env = awsb.parse_dotenv((HERE.parents[1] / ".env").read_text())
    assert "ESPRESSO_ETH_MNEMONIC" in env
    assert "ESPRESSO_STAKE_TABLE_PROXY_ADDRESS" in env
    assert env["ESPRESSO_OPS_TIMELOCK_ADMIN"] == env["ESPRESSO_ETH_MULTISIG_ADDRESS"]


def node_env(name: str, nodes: int = 5) -> dict[str, str]:
    cfg = small_cfg(nodes=nodes, submit=nodes - 1)
    spec = next(h for h in awsb.plan_hosts(cfg) if h["name"] == name)
    return parse_env(awsb.render_node_env(spec, fleet(nodes), awsb.pg_endpoint()))


def test_journal_max_bytes_is_set_only_for_journal_storage():
    spec = next(h for h in awsb.plan_hosts(small_cfg()) if h["name"] == "node1")
    env = {
        storage: parse_env(
            awsb.render_node_env(spec, fleet(2), None, consensus_storage=storage)
        )
        for storage in ("fs", "journal")
    }
    assert "ESPRESSO_NODE_JOURNAL_MAX_BYTES" not in env["fs"]
    assert int(env["journal"]["ESPRESSO_NODE_JOURNAL_MAX_BYTES"]) > 0
    assert env["fs"]["ESPRESSO_NODE_STORAGE_PATH"] == "/store/espresso"


def test_sqlite_node0_env_selects_the_embedded_db_and_has_no_postgres():
    spec = next(h for h in awsb.plan_hosts(small_cfg()) if h["name"] == "node0")
    env = parse_env(awsb.render_node_env(spec, fleet(2), None, query_engine="sqlite"))
    assert env["ESPRESSO_NODE_EMBEDDED_DB"] == "true"
    assert not [key for key in env if "POSTGRES" in key]
    assert "ESPRESSO_NODE_EMBEDDED_DB" not in node_env("node0")
    assert "ESPRESSO_NODE_JOURNAL_IGNORE_EXISTING" not in env
    journal = parse_env(
        awsb.render_node_env(
            spec,
            fleet(2),
            None,
            consensus_storage="journal",
            query_engine="sqlite",
        )
    )
    assert journal["ESPRESSO_NODE_JOURNAL_IGNORE_EXISTING"] == "true"


def test_node_env_streams_l1_heads_over_websocket():
    env = node_env("node1")
    http = env["ESPRESSO_L1_PROVIDER"]
    assert env["ESPRESSO_L1_WS_PROVIDER"] == http.replace("http://", "ws://", 1)


def test_node_env_overrides_reach_every_node():
    extra = ("ESPRESSO_QUERY_PAYLOAD_DIR=/payload", "RUST_LOG=debug,a=b")
    hosts = fleet(5)
    for spec in awsb.plan_hosts(small_cfg(nodes=5, submit=4)):
        if spec["role"] == "ctl":
            continue
        pg = awsb.pg_endpoint() if spec["role"] == "query" else None
        text = awsb.render_node_env(spec, hosts, pg, extra)
        env = parse_env(text)
        assert env["ESPRESSO_QUERY_PAYLOAD_DIR"] == "/payload"
        assert env["RUST_LOG"] == "debug,a=b"
        assert text.count("RUST_LOG=") == 1


def node_env_config(*flags: str) -> Any:
    return awsb.config_from_args(awsb.parse_args(["run", "--tag", "x", *flags]))


def test_node_env_flag():
    assert node_env_config().node_env == ()
    flags = ("--node-env", "A=1", "--node-env", "B=")
    assert node_env_config(*flags).node_env == ("A=1", "B=")
    with pytest.raises(awsb.Refused, match="repeats A"):
        node_env_config("--node-env", "A=1", "--node-env", "A=2")


def test_consensus_storage_flag_defaults_to_fs_and_rejects_other_values():
    assert node_env_config().consensus_storage == "fs"
    assert node_env_config("--consensus-storage", "journal").consensus_storage == (
        "journal"
    )
    with pytest.raises(SystemExit):
        node_env_config("--consensus-storage", "rocksdb")


def test_consensus_storage_differs_in_the_config_hash_only_when_not_journal():
    hosts = awsb.plan_hosts(small_cfg())

    def digest(storage: str) -> str:
        cfg = small_cfg(consensus_storage=storage)
        return awsb.run_config_hash(cfg, hosts, {}, b"genesis")

    assert digest("fs") != digest("journal")
    assert digest("journal") == awsb.run_config_hash(
        dataclasses.replace(small_cfg(), consensus_storage="journal"),
        hosts,
        {},
        b"genesis",
    )


def test_manifest_config_without_consensus_storage_loads_as_journal():
    saved = awsb.config_to_json(small_cfg())
    del saved["consensus_storage"]
    assert awsb.config_from_manifest(saved).consensus_storage == "journal"


def test_submit_workers_flag_reaches_the_load_config():
    assert node_env_config().load.workers == awsb.SUBMIT_WORKERS
    assert node_env_config("--submit-workers", "64").load.workers == 64


@pytest.mark.parametrize("bad", ["0", "-1", "x"])
def test_submit_workers_flag_rejects_non_positive(bad):
    with pytest.raises(SystemExit):
        node_env_config("--submit-workers", bad)


def test_namespaces_flag_reaches_the_load_config():
    assert node_env_config().load.namespaces == (10000, 10015)
    assert node_env_config("--namespaces", "80").load.namespaces == (10000, 10079)
    assert node_env_config("--namespaces", "1").load.namespaces == (10000, 10000)


@pytest.mark.parametrize("bad", ["0", "-1", "x"])
def test_namespaces_flag_rejects_non_positive(bad):
    with pytest.raises(SystemExit):
        node_env_config("--namespaces", bad)


@pytest.mark.parametrize("bad", ["A", "=1", "1A=2", "A B=1"])
def test_node_env_flag_rejects_malformed(bad):
    with pytest.raises(SystemExit):
        node_env_config("--node-env", bad)


def test_key_index_offset_by_twenty():
    assert node_env("node0")["ESPRESSO_NODE_KEY_INDEX"] == "20"
    assert node_env("node3")["ESPRESSO_NODE_KEY_INDEX"] == "23"


def test_storage_vars_by_role():
    validator, query = node_env("node1"), node_env("node0")
    assert "ESPRESSO_NODE_POSTGRES_HOST" not in validator
    assert validator["ESPRESSO_NODE_STORAGE_PATH"] == "/store/espresso"
    assert query["ESPRESSO_NODE_POSTGRES_HOST"] == "127.0.0.1"
    assert query["ESPRESSO_NODE_POSTGRES_DATABASE"] == "espresso"


def test_payload_dir_is_opt_in_through_node_env():
    assert "ESPRESSO_QUERY_PAYLOAD_DIR" not in node_env("node0")
    text = awsb.render_node_env(
        host("node0", "query"),
        fleet(5),
        awsb.pg_endpoint(),
        (f"ESPRESSO_QUERY_PAYLOAD_DIR={awsb.PAYLOAD_CONTAINER_DIR}",),
    )
    assert parse_env(text)["ESPRESSO_QUERY_PAYLOAD_DIR"] == "/payload"


@pytest.mark.parametrize("index", [0, 1])
def test_state_peers_follow_peers(index):
    hosts = fleet(5)
    expected = ",".join(
        f"http://{hosts[f'node{j}']['private_ip']}:{awsb.NODE_API_PORT}"
        for j in awsb.peers(index, 5)
    )
    assert node_env(f"node{index}")["ESPRESSO_NODE_STATE_PEERS"] == expected


def test_state_peers_omitted_when_empty():
    assert "ESPRESSO_NODE_STATE_PEERS" not in node_env("node1", nodes=2)


def test_advertise_addresses_use_private_ip_and_dns():
    env = node_env("node2")
    assert (
        env["ESPRESSO_NODE_CLIQUENET_ADVERTISE_ADDRESS"]
        == f"10.0.0.4:{awsb.CLIQUENET_PORT}"
    )
    assert (
        env["ESPRESSO_NODE_LIBP2P_ADVERTISE_ADDRESS"]
        == f"ip-10-0-0-4.eu-west-1.compute.internal:{awsb.LIBP2P_PORT}"
    )


@pytest.fixture
def ctl_env() -> dict[str, str]:
    cfg = small_cfg(nodes=5, submit=4)
    return parse_env(awsb.render_ctl_env(fleet(5), cfg, awsb.parse_dotenv(DOTENV_TEXT)))


def test_ctl_env_unsets_proxy_addresses(ctl_env):
    assert not set(awsb.PROXY_ADDRESS_KEYS) & ctl_env.keys()
    assert "ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS" in ctl_env


def test_ctl_env_orchestrator_nodes_and_builder_disabled(ctl_env):
    assert ctl_env["ESPRESSO_ORCHESTRATOR_NUM_NODES"] == "5"
    assert ctl_env["ESPRESSO_ORCHESTRATOR_BUILDER_URLS"] == "http://localhost:1"
    assert ctl_env["ESPRESSO_ORCHESTRATOR_BUILDER_TIMEOUT"] == "100ms"


def test_ctl_env_has_no_unsubstituted_reference(ctl_env):
    assert not [value for value in ctl_env.values() if "$" in value]


# REQ:awsbench-user-data
def test_user_data_ttl_line_is_first():
    text = awsb.render_user_data(host("ctl", "ctl"), fake_images(), ttl_s=930)
    lines = [
        line
        for line in text.splitlines()
        if line.strip() and not line.startswith(("#", "set "))
    ]
    assert lines[0] == "shutdown -P +16"


def test_user_data_pulls_every_image_by_digest():
    images = fake_images()
    text = awsb.render_user_data(host("node0", "query"), images, ttl_s=60)
    for name in awsb.ROLE_IMAGES["query"]:
        assert f"docker pull {images[name]['ref']}@{images[name]['digest']}" in text


def test_user_data_has_no_unsubstituted_placeholder():
    text = awsb.render_user_data(host("ctl", "ctl"), fake_images(), ttl_s=60)
    found = set(re.findall(r"\$\{?[A-Za-z_]\w*", text))
    # $digests/$chrony: ready.json's jq filter. $name/$digest: the per-image jq merge that
    # records what was actually pulled. $pulled: the bash variable holding it. None are
    # leftover Template placeholders.
    assert found <= {
        "$digests",
        "$chrony",
        "$uv",
        "$pythons",
        "$name",
        "$digest",
        "$pulled",
    }


def test_anvil_uses_entrypoint_and_binds_every_interface():
    script = awsb.render_start_sh(host("ctl", "ctl"), fake_images(), 32768)
    assert "--entrypoint anvil" in script
    assert "--host 0.0.0.0" in script


START_SH_GOLDEN_DIR = Path(__file__).parent / "golden"
START_SH_GOLDEN_CASES = {
    "ctl": (host("ctl", "ctl"), {}),
    "validator": (host("node1", "validator"), {}),
    "validator-journal": (host("node1", "validator"), {"consensus_storage": "journal"}),
    "validator-trace": (host("node1", "validator"), {"leader_trace": True}),
    "query-postgres": (host("node0", "query"), {}),
    "query-rds": (host("node0", "query"), {"query_db": "rds"}),
    "query-sqlite": (host("node0", "query"), {"query_engine": "sqlite"}),
    "query-sqlite-volume": (
        host("node0", "query"),
        {"query_engine": "sqlite", "query_db": "volume"},
    ),
    "query-sqlite-tmpfs": (
        host("node0", "query"),
        {"query_engine": "sqlite", "query_db": "tmpfs", "leader_trace": True},
    ),
}


# REQ:awsbench-start-sh-golden
# TEST:start-sh-golden-ok
@pytest.mark.parametrize("case", START_SH_GOLDEN_CASES)
def test_start_sh_matches_golden(case):
    spec, kwargs = START_SH_GOLDEN_CASES[case]
    script = awsb.render_start_sh(spec, fake_images(), 32768, **kwargs)
    assert script == (START_SH_GOLDEN_DIR / f"start-{case}.sh").read_text()


def test_validator_start_sh_uses_storage_journal_only():
    script = awsb.render_start_sh(
        host("node1", "validator"), fake_images(), 32768, consensus_storage="journal"
    )
    assert "-- storage-journal -- http" in script
    assert "storage-sql" not in script
    assert "--name postgres" not in script
    assert "/opt/bench/genesis.toml:/opt/bench/genesis.toml:ro" in script
    assert "payload" not in script


def test_query_start_sh_mounts_payload_dir_from_pg_volume():
    for mode in ("colocated", "volume", "rds"):
        script = awsb.render_start_sh(
            host("node0", "query"), fake_images(), 32768, mode
        )
        assert script.index("mkdir -p /data/pg/payload") < script.index(
            "--name espresso-node"
        )
        assert "-v /data/pg/payload:/payload" in script
        assert "--tmpfs" not in script


def test_validators_get_provisioned_root_disks():
    for spec in awsb.plan_hosts(small_cfg(nodes=3, submit=2)):
        if spec["role"] == "validator":
            assert spec["root_iops"] == awsb.VALIDATOR_ROOT_IOPS
            assert spec["root_mbps"] == awsb.VALIDATOR_ROOT_MBPS


def test_query_start_sh_adds_storage_sql_and_postgres():
    script = awsb.render_start_sh(
        host("node0", "query"), fake_images(), 32768, consensus_storage="journal"
    )
    assert "-- storage-journal -- storage-sql" in script
    assert "--name postgres" in script
    assert script.index("shared_preload_libraries") > script.index("@sha256")
    assert "-c shared_buffers=8GB" in script


def test_node_summary_names_the_storage_modules_and_every_run_setting():
    cfg = small_cfg(
        consensus_storage="journal",
        query_db="volume",
        max_block_size="20mb",
        leader_trace=True,
        node_env=("A=1", "B=2"),
    )
    assert awsb.format_node_summary(cfg) == [
        "storage validators: consensus storage-journal",
        "storage query:      consensus storage-journal, query storage-sql (postgres, volume)",
        "nodes: 2 (1 query); max block 20mb; submit 1 nodes; leader-trace on",
        "node-env: A=1 B=2",
    ]
    fs = awsb.format_node_summary(small_cfg())
    assert fs[:2] == [
        "storage validators: consensus storage-fs",
        "storage query:      consensus storage-sql, query storage-sql (postgres, colocated)",
    ]
    assert fs[2].endswith("leader-trace off")
    assert fs[3] == "node-env: none"


def test_fs_validator_start_sh_uses_storage_fs_only():
    script = awsb.render_start_sh(host("node1", "validator"), fake_images(), 32768)
    assert "-- storage-fs -- http" in script
    assert "storage-journal" not in script
    assert "storage-sql" not in script


def test_sqlite_query_start_sh_leaves_storage_sql_to_the_entrypoint():
    for storage, modules in (
        ("journal", "-- storage-journal -- http -- query"),
        ("fs", "/bin/espresso-node -- http -- query"),
    ):
        script = awsb.render_start_sh(
            host("node0", "query"),
            fake_images(),
            32768,
            consensus_storage=storage,
            query_engine="sqlite",
        )
        assert modules in script
        assert "storage-sql" not in script
        assert "--name postgres" not in script
        assert "/data/pg:/var/lib/postgresql" not in script
        assert "-v /data/pg/payload:/payload" in script
        assert "--tmpfs" not in script


def test_sqlite_tmpfs_mounts_ram_over_the_database_directory():
    script = awsb.render_start_sh(
        host("node0", "query"),
        fake_images(),
        32768,
        "tmpfs",
        query_engine="sqlite",
    )
    assert "--tmpfs /store/espresso/sqlite:size=8g" in script
    assert script.index("--tmpfs") < script.index("@sha256")
    validator = awsb.render_start_sh(host("node1", "validator"), fake_images(), 32768)
    assert "--tmpfs" not in validator


def test_sqlite_volume_mounts_the_volume_directory_after_it_is_created():
    script = awsb.render_start_sh(
        host("node0", "query"),
        fake_images(),
        32768,
        "volume",
        query_engine="sqlite",
    )
    assert "--tmpfs" not in script and "--name postgres" not in script
    assert "-v /data/pg/sqlite:/store/espresso/sqlite" in script
    assert script.index("mkdir -p /data/pg/sqlite") < script.index(
        "--name espresso-node"
    )


def test_sqlite_topology_names_no_postgres():
    roles = awsb.topology(fleet(2), "sqlite")["roles"]
    assert roles["node0"] == "validator, query, sqlite"
    assert awsb.topology(fleet(2), "postgres")["roles"]["node0"].endswith("postgres")


def test_fs_query_start_sh_uses_storage_sql_only():
    script = awsb.render_start_sh(host("node0", "query"), fake_images(), 32768)
    assert "-- storage-sql -- http -- query" in script
    assert "storage-fs" not in script
    assert "storage-journal" not in script
    assert "--name postgres" in script


def test_query_start_sh_scales_postgres_with_the_host_memory():
    script = awsb.render_start_sh(host("node0", "query"), fake_images(), 16384)
    assert "-c shared_buffers=4GB" in script
    assert "-c effective_cache_size=12GB" in script


# REQ:awsbench-hostmon-parsers
def test_diskstats_parses_known_columns():
    line = "   8       0 nvme0n1 100 0 2000 5 200 0 4000 10 0 20 20\n"
    assert awsb.diskstats(line)["nvme0n1"] == {
        "read_bytes": 2000 * 512,
        "write_bytes": 4000 * 512,
        "io_ticks_ms": 20,
        "reads": 100,
        "writes": 200,
        "read_ms": 5,
        "write_ms": 10,
        "in_flight": 0,
        "weighted_ms": 20,
    }


def test_netdev_parses_rx_and_tx_bytes():
    text = (
        "Inter-|   Receive                                                |  Transmit\n"
        " face |bytes    packets errs drop fifo frame compressed multicast|"
        "bytes    packets errs drop fifo colls carrier compressed\n"
        "    lo:  100    1    0    0    0     0          0         0  "
        "  100    1    0    0    0     0       0          0\n"
        "  eth0: 5000    5    0    0    0     0          0         0 "
        " 7000    7    0    0    0     0       0          0\n"
    )
    assert awsb.netdev(text) == {
        "eth0": {"rx_bytes": 5000, "tx_bytes": 7000},
        "lo": {"rx_bytes": 100, "tx_bytes": 100},
    }


def test_container_stats_reads_scope_files(tmp_path):
    scope = tmp_path / "system.slice" / "docker-abc123.scope"
    scope.mkdir(parents=True)
    (scope / "cpu.stat").write_text("usage_usec 2000000\n")
    # file (page cache) dwarfs anon here; container_stats must report anon, not the sum.
    (scope / "memory.stat").write_text("anon 104857600\nfile 900000000\n")
    stats = awsb.container_stats({"espresso-node": "abc123"}, tmp_path)
    assert stats == {"espresso-node": {"cpu_s": 2.0, "rss": 104857600}}


def test_container_stats_skips_missing_scope(tmp_path):
    assert awsb.container_stats({"gone": "deadbeef"}, tmp_path) == {}


def test_leader_trace_flag_is_off_by_default():
    assert node_env_config().leader_trace is False
    assert node_env_config("--leader-trace").leader_trace is True
    assert node_env_config("--leader-trace", "--no-leader-trace").leader_trace is False


def test_leader_trace_differs_in_the_config_hash_only_when_on():
    hosts = awsb.plan_hosts(small_cfg())

    def digest(**kw) -> str:
        return awsb.run_config_hash(small_cfg(**kw), hosts, {}, b"genesis")

    assert digest() == digest(leader_trace=False)
    assert digest() != digest(leader_trace=True)


def test_manifest_config_without_leader_trace_loads_as_off():
    saved = awsb.config_to_json(small_cfg())
    del saved["leader_trace"]
    assert awsb.config_from_manifest(saved).leader_trace is False


@pytest.mark.parametrize("role", ["query", "validator"])
def test_leader_trace_start_sh_mounts_a_host_dir_for_node_roles(role: str):
    spec = host("node0", role)
    on = awsb.render_start_sh(spec, fake_images(), 32768, leader_trace=True)
    assert "mkdir -p /opt/bench/trace\n" in on
    assert "-v /opt/bench/trace:/trace" in on
    assert on.index("mkdir -p /opt/bench/trace") < on.rindex("docker create")
    off = awsb.render_start_sh(spec, fake_images(), 32768)
    assert "trace" not in off


def test_leader_trace_start_sh_leaves_ctl_alone():
    script = awsb.render_start_sh(
        host("ctl", "ctl"), fake_images(), 32768, leader_trace=True
    )
    assert "trace" not in script


def test_submit_nodes_defaults_to_every_node():
    assert node_env_config().load.submit_nodes == 5
    assert node_env_config("--nodes", "7").load.submit_nodes == 7


def test_submit_nodes_flag_reaches_the_load_config():
    assert node_env_config("--submit-nodes", "5").load.submit_nodes == 5
    assert node_env_config("--nodes", "3", "--submit-nodes", "1").load.submit_nodes == 1


def test_submit_nodes_above_nodes_is_refused():
    with pytest.raises(awsb.Refused, match="--submit-nodes"):
        awsb.plan_hosts(node_env_config("--submit-nodes", "6"))


def test_submit_nodes_zero_is_an_argument_error():
    with pytest.raises(SystemExit):
        node_env_config("--submit-nodes", "0")


def test_manifest_config_round_trips_submit_nodes():
    cfg = small_cfg(nodes=3, submit=3)
    saved = json.loads(json.dumps(awsb.config_to_json(cfg)))
    assert awsb.config_from_manifest(saved).load.submit_nodes == 3


def search_cfg(*flags: str, **fields: Any) -> Any:
    argv = ["plan", "--tag", "x", "--nodes", "5", "--search", *flags]
    cfg = awsb.config_from_args(awsb.parse_args(argv))
    return replace(cfg, **fields)


# TEST:aws-search-flags-ok
@pytest.mark.parametrize(
    ("flags", "start", "step_s", "cap_s", "tx_timeout_s"),
    [
        ((), 100.0, 60, 60.0, 60),
        (("150",), 150.0, 60, 60.0, 60),
        (("150", "--cap-s", "120"), 150.0, 60, 120.0, 60),
        (("150", "--step-s", "30", "--tx-timeout-s", "90"), 150.0, 30, 60.0, 90),
    ],
)
def test_search_flags_build_the_config(flags, start, step_s, cap_s, tx_timeout_s):
    cfg = search_cfg(*flags)
    assert cfg.search == netbench.SearchConfig(start_mb_s=start)
    load = cfg.load
    assert (load.step_s, load.cap_s, load.tx_timeout_s) == (step_s, cap_s, tx_timeout_s)


def test_search_flags_override_the_budgets():
    cfg = search_cfg(
        "--resolution-mb-s", "5", "--max-probes", "8", "--offered-gb", "90"
    )
    assert cfg.search == netbench.SearchConfig(
        resolution_mb_s=5.0, max_probes=8, offered_gb=90.0
    )


def test_without_search_the_legacy_defaults_hold():
    args = awsb.parse_args(["plan", "--tag", "x"])
    cfg = awsb.config_from_args(args)
    assert cfg.search is None
    legacy = awsb.aws_load()
    load = cfg.load
    assert (load.steps, load.step_s, load.cap_s, load.tx_timeout_s) == (
        legacy.steps,
        legacy.step_s,
        legacy.cap_s,
        legacy.tx_timeout_s,
    )


@pytest.mark.parametrize("flag", ["--max-probes", "--offered-gb", "--resolution-mb-s"])
def test_search_budget_flags_need_search(flag):
    args = awsb.parse_args(["plan", "--tag", "x", flag, "5"])
    with pytest.raises(awsb.Refused, match="needs --search"):
        awsb.config_from_args(args)


# TEST:aws-check-search-fails
def test_search_refuses_two_steps():
    args = awsb.parse_args(
        ["plan", "--tag", "x", "--search", "150", "--steps", "100,200"]
    )
    with pytest.raises(awsb.Refused, match="--search takes one --steps value"):
        awsb.config_from_args(args)


def test_search_refuses_keep_going():
    cfg = search_cfg("150")
    kept = replace(cfg, load=replace(cfg.load, keep_going=True))
    with pytest.raises(awsb.Refused, match="--keep-going"):
        awsb.check_search(kept)


def test_search_refuses_a_volume_too_small_for_the_payload():
    cfg = search_cfg("150", "--offered-gb", "200", nodes=3, db_modes=("volume",))
    with pytest.raises(awsb.Refused, match="Postgres volume"):
        awsb.plan_hosts(cfg)


# TEST:rds-search-ok
def test_search_checks_the_rds_volume_with_the_same_factor():
    ok = search_cfg(
        "150", db_modes=("rds",), pg_iops=awsb.RDS_IOPS, pg_mbps=awsb.RDS_MBPS
    )
    awsb.check_search(ok)
    too_big = replace(ok, search=replace(ok.search, offered_gb=300.0), nodes=3)
    with pytest.raises(awsb.Refused, match="Postgres volume"):
        awsb.check_search(too_big)


# TEST:offered-gb-small-fails
def test_search_refuses_an_offer_below_the_first_probe():
    with pytest.raises(awsb.Refused, match="--offered-gb"):
        awsb.check_search(search_cfg("150", "--offered-gb", "10"))


# TEST:aws-disk-sizing-ok
@pytest.mark.parametrize(
    ("db_modes", "validator_gb", "query_gb"),
    [(("colocated",), 260, 554), (("volume",), 260, 554), (("rds",), 260, 554)],
)
def test_search_sizes_disks_from_the_offered_gb(db_modes, validator_gb, query_gb):
    cfg = search_cfg("--offered-gb", "120", db_modes=db_modes)
    assert awsb.node_root_gb(cfg, query=False) == validator_gb
    assert awsb.node_root_gb(cfg, query=True) == query_gb


def test_payload_factor():
    assert awsb.payload_factor(3) == 2.0
    assert awsb.payload_factor(5) == pytest.approx(1.6)


def test_explicit_root_gb_wins_over_the_search_sizing():
    assert awsb.node_root_gb(search_cfg(root_gb="500"), query=True) == 500


# TEST:aws-time-cost-ok
def test_search_load_seconds():
    load = netbench.BenchConfig(step_s=60, warmup_s=60, tx_timeout_s=60)
    search = netbench.SearchConfig()
    assert (
        awsb.load_seconds(load, search, worst=True)
        == 60 + 12 * (2 * 60 + netbench.DRAIN_MAX_S + netbench.DRAIN_GRACE_S) + 60
    )
    assert (
        awsb.load_seconds(load, search, worst=False)
        == 60 + 11 * 60 + 3 * awsb.DRAIN_EXPECTED_S + 60
    )


def test_search_measure_seconds_use_the_worst_load_for_the_bound():
    cfg = search_cfg("150")
    expected_s, worst_s = awsb._measure_seconds(cfg)
    assert expected_s == (
        awsb.SERVICES_S
        + awsb.load_seconds(cfg.load, cfg.search, worst=False)
        + awsb.READY_EXPECTED_S
        + awsb.COLLECT_EXPECTED_S
    )
    assert worst_s == (
        awsb.SERVICES_S
        + awsb.load_seconds(cfg.load, cfg.search, worst=True)
        + awsb.READY_TIMEOUT_S
        + awsb.COLLECT_MAX_S
    )


def test_search_summary_line():
    assert awsb.format_search_summary(search_cfg()) == (
        "search from 100 MB/s: climb x1.25, bisect to 5 MB/s, confirm 120 s; "
        "at most 12 probes, 150 GB offered"
    )


def test_estimate_logs_the_search_summary(caplog):
    cfg = search_cfg("150")
    hosts = awsb.plan_hosts(cfg)
    with caplog.at_level("INFO", logger="aws-bench"):
        awsb.estimate_or_refuse(
            replace(cfg, max_usd=1000.0),
            hosts,
            Path("."),
            0.0,
            INSTANCE_PRICES,
        )
    summary = awsb.format_search_summary(cfg)
    messages = [r.getMessage() for r in caplog.records]
    assert messages[-1] == summary
    assert messages[-5:-1] == awsb.format_node_summary(cfg)
    assert messages[-6].startswith("cost: expected")


# TEST:aws-config-hash-ok
def test_search_changes_the_run_config_hash():
    ramp = awsb.RunConfig(tag="x")
    searched = replace(ramp, search=netbench.SearchConfig())
    other = replace(ramp, search=netbench.SearchConfig(start_mb_s=150.0))
    args = ([], {}, b"")
    hashes = {awsb.run_config_hash(c, *args) for c in (ramp, searched, other)}
    assert len(hashes) == 3


def test_the_ramp_run_config_hash_ignores_the_search_field():
    cfg = awsb.RunConfig(tag="x", consensus_storage="journal")
    assert awsb.run_config_hash(cfg, [], {}, b"") == netbench.config_hash(
        cfg.load, [b"", b"{}", b"[]", b"colocated", b"query-nodes=1"]
    )


def test_search_survives_the_manifest_round_trip():
    cfg = search_cfg("150")
    assert awsb.config_from_manifest(awsb.config_to_json(cfg)).search == cfg.search
    ramp = awsb.config_to_json(awsb.RunConfig(tag="x"))
    del ramp["search"]
    assert awsb.config_from_manifest(ramp).search is None


def test_search_config_reads_the_agent_key():
    assert awsb.search_config(None) is None
    saved = dataclasses.asdict(netbench.SearchConfig(start_mb_s=150.0))
    assert awsb.search_config(saved) == netbench.SearchConfig(start_mb_s=150.0)


@pytest.mark.parametrize("db_modes", [("colocated",), ("volume",), ("rds",)])
@pytest.mark.parametrize("nodes", [3, 5, 100])
def test_search_node0_journal_holds_the_offer_below_the_noisy_mark(db_modes, nodes):
    cfg = search_cfg("--offered-gb", "150", db_modes=db_modes, nodes=nodes)
    host = {"role": "query", "root_gb": awsb.node_root_gb(cfg, query=True)}
    stored = awsb.payload_factor(nodes) * 150 * 1_000_000_000
    assert stored <= awsb.JOURNAL_NOISY_FRACTION * awsb._journal_max_bytes(host)
