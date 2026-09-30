import argparse
import json
import re
import shutil
import subprocess
import threading
from collections.abc import Iterator
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import (
    DOTENV_TEXT,
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
    shot_estimate,
    sts_response,
)

HERE = Path(__file__).parent
TF_DIR = Path("/tmp/aws-bench/run1/terraform")


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
    ("keep_going", "expected"),
    [
        (False, 60 + 3 * 30 + 2 * 30 + netbench.DRAIN_SLACK_S),
        (True, 60 + 2 * 30 + netbench.CATCHUP_TIMEOUT_S + 30),
    ],
)
def test_load_seconds(keep_going, expected):
    load = netbench.BenchConfig(
        steps=(4.0, 8.0), step_s=30, warmup_s=60, tx_timeout_s=30, keep_going=keep_going
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
    assert estimate["expected_s"] == 1695.0
    assert estimate["ttl_s"] == 3315.0
    assert estimate["expected_usd"] == pytest.approx(0.858907, abs=1e-5)
    assert estimate["bound_usd"] == pytest.approx(1.754221, abs=1e-5)


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
    assert not {"account_id", "region", "profile"} & tfvars.keys()


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
    assert awsb.caller_account(runner) == awsb.ACCOUNT


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
    assert awsb.capable_az(runner, {"c8g.2xlarge", "c8g.4xlarge"}) == "eu-west-1b"


def test_no_capable_az_refuses():
    runner = FakeRunner()
    runner.respond("Values=c8g.2xlarge", lambda _: offerings("eu-west-1a"))
    runner.respond("Values=c8g.4xlarge", lambda _: offerings("eu-west-1b"))
    with pytest.raises(awsb.Refused, match="no AZ in eu-west-1 offers"):
        awsb.capable_az(runner, {"c8g.2xlarge", "c8g.4xlarge"})


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
    monkeypatch.setattr(awsb, "resolve_image", lambda _http_get, ref: fake_image(ref))
    result = awsb.preflight(FakeSystem(run=runner), cfg, awsb.plan_hosts(cfg))
    assert result["az"] == "eu-west-1a"
    assert result["ami_id"] == "ami-0abc"
    assert set(result["images"]) == set(awsb.image_refs(cfg))


@pytest.mark.parametrize(
    ("stage", "stderr", "message"),
    [
        (
            "apply",
            "Error: creating\nVcpuLimitExceeded: x\n\n",
            "tofu apply failed: VcpuLimitExceeded: x",
        ),
        ("destroy", "boom", "tofu destroy failed: boom"),
    ],
)
def test_tofu_failure_raises_with_stage_and_last_line(stage, stderr, message):
    runner = FakeRunner(
        {("tofu", f"-chdir={TF_DIR}", stage): completed(returncode=1, stderr=stderr)}
    )
    tf = awsb.Terraform(runner, TF_DIR, env={})
    with pytest.raises(awsb.TfFailed) as err:
        getattr(tf, stage)()
    assert err.value.stage == stage
    assert str(err.value) == message


def resolve_with(registry: FakeRegistry) -> dict:
    return awsb.resolve_image(FakeSystem(http=registry).http_get, registry.ref)


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
    rendered = awsb.render_genesis((HERE / "genesis.toml").read_bytes(), n=n).decode()
    assert f"stake_table_capacity = {capacity}" in rendered
    assert re.search(rf"^capacity = {capacity}$", rendered, re.MULTILINE)


def test_genesis_raises_if_template_shape_changes():
    with pytest.raises(ValueError):
        awsb.render_genesis(b"no capacity fields here", n=5)


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
    script = awsb.render_start_sh(host("ctl", "ctl"), fake_images())
    assert "--entrypoint anvil" in script
    assert "--host 0.0.0.0" in script


def test_validator_start_sh_uses_storage_journal_only():
    script = awsb.render_start_sh(host("node1", "validator"), fake_images())
    assert "-- storage-journal -- http" in script
    assert "storage-sql" not in script
    assert "--name postgres" not in script
    assert "/opt/bench/genesis.toml:/opt/bench/genesis.toml:ro" in script


def test_query_start_sh_adds_storage_sql_and_postgres():
    script = awsb.render_start_sh(host("node0", "query"), fake_images())
    assert "-- storage-journal -- storage-sql" in script
    assert "--name postgres" in script
    assert script.index("shared_preload_libraries") > script.index("@sha256")


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
