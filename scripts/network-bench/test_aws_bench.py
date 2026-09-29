"""Tests for `aws-bench`: cost estimate, budget refusal, and `plan --offline` rendering.

Side effects go through a `FakeRunner` that maps an argv prefix to a canned
`subprocess.CompletedProcess`, and records every call, so a refused plan can be shown to have
made no `aws` call at all.

    just py::test
"""

import importlib.util
import json
import os
import shutil
import subprocess
import tempfile
import threading
import unittest
import unittest.mock
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib.machinery import SourceFileLoader
from pathlib import Path

import netbench

SCRIPT = Path(__file__).with_name("aws-bench")
_spec = importlib.util.spec_from_loader(
    "aws_bench", SourceFileLoader("aws_bench", str(SCRIPT))
)
assert _spec is not None
awsb = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(awsb)


def completed(
    stdout: str = "", returncode: int = 0, stderr: str = ""
) -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess(
        args=[], returncode=returncode, stdout=stdout, stderr=stderr
    )


class FakeRunner:
    """Maps an argv prefix to a canned `CompletedProcess`; records every call. An argv with no
    matching prefix raises, so a test can prove a refused plan never reached `aws` or `tofu`."""

    def __init__(
        self,
        responses: dict[tuple[str, ...], subprocess.CompletedProcess] | None = None,
    ):
        self.responses = responses or {}
        self.calls: list[list[str]] = []

    def __call__(self, argv: list[str]) -> subprocess.CompletedProcess:
        self.calls.append(argv)
        for prefix, result in self.responses.items():
            if tuple(argv[: len(prefix)]) == prefix:
                return result
        raise AssertionError(f"unexpected command: {argv!r}")

    def ran(self, *prefix: str) -> bool:
        return any(tuple(call[: len(prefix)]) == prefix for call in self.calls)


def price_response(usd_hour: float) -> subprocess.CompletedProcess:
    product = json.dumps(
        {
            "terms": {
                "OnDemand": {
                    "x": {
                        "priceDimensions": {
                            "y": {
                                "unit": "Hrs",
                                "pricePerUnit": {"USD": str(usd_hour)},
                            }
                        }
                    }
                }
            }
        }
    )
    return completed(stdout=json.dumps({"PriceList": [product]}))


def parse_plan_args(argv: list[str]) -> "awsb.argparse.Namespace":
    full = ["plan", *argv]
    args = awsb.parse_args(full)
    args.argv = full
    return args


def cmd_plan_exit(args: "awsb.argparse.Namespace", run: "awsb.Runner") -> int:
    """Mirrors `main`'s single `Refused` catch, since these tests call `cmd_plan` directly."""
    try:
        return awsb.cmd_plan(args, run=run)
    except awsb.Refused:
        return awsb.EXIT_REFUSED


def sts_response(account: str) -> subprocess.CompletedProcess:
    return completed(stdout=json.dumps({"Account": account}))


STS_CALL = ("aws", "--profile", "timeboost-dev", "sts", "get-caller-identity")


def two_node_prices() -> "awsb.Prices":
    return {
        "instances": {
            "c8g.4xlarge": {"usd_hour": 0.71, "source": "test"},
            "c8g.2xlarge": {"usd_hour": 0.355, "source": "test"},
        }
    }


# REQ:awsbench-topology
class PeersTest(unittest.TestCase):
    def test_excludes_self_and_node0(self):
        for n in range(3, 9):
            for i in range(1, n):
                result = awsb.peers(i, n)
                self.assertNotIn(i, result)
                self.assertNotIn(0, result)
                self.assertEqual(len(result), min(3, n - 2))

    def test_deterministic(self):
        self.assertEqual(awsb.peers(1, 5), [2, 3, 4])
        self.assertEqual(awsb.peers(4, 5), [1, 2, 3])

    def test_node0_has_no_peers(self):
        for n in range(1, 10):
            self.assertEqual(awsb.peers(0, n), [])

    # EDGE:awsbench-two-nodes
    def test_two_nodes_empty(self):
        self.assertEqual(awsb.peers(1, 2), [])
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        self.assertEqual(awsb.plan_peers(hosts), {"node0": [], "node1": []})


class PlanHostsTest(unittest.TestCase):
    def test_roles_and_count(self):
        hosts = awsb.plan_hosts(awsb.RunConfig(tag="x", nodes=5))
        self.assertEqual(
            [h["name"] for h in hosts],
            ["ctl", "node0", "node1", "node2", "node3", "node4"],
        )
        self.assertEqual(hosts[0]["role"], "ctl")
        self.assertEqual(hosts[1]["role"], "query")
        self.assertTrue(all(h["role"] == "validator" for h in hosts[2:]))

    def test_query_node_gets_pg_iops(self):
        hosts = awsb.plan_hosts(
            awsb.RunConfig(
                tag="x",
                nodes=3,
                pg_iops=6000,
                pg_mbps=500,
                load=netbench.BenchConfig(submit_nodes=2),
            )
        )
        node0 = next(h for h in hosts if h["name"] == "node0")
        node1 = next(h for h in hosts if h["name"] == "node1")
        self.assertEqual(node0["root_iops"], 6000)
        self.assertEqual(node0["root_mbps"], 500)
        self.assertEqual(node1["root_iops"], awsb.GP3_BASELINE_IOPS)

    def test_rejects_single_node(self):
        with self.assertRaises(awsb.Refused):
            awsb.plan_hosts(awsb.RunConfig(tag="x", nodes=1))

    def test_rejects_submit_nodes_out_of_range(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=2)
        )
        with self.assertRaises(awsb.Refused):
            awsb.plan_hosts(cfg)


class PhaseSecondsTest(unittest.TestCase):
    def test_load_seconds(self):
        load = netbench.BenchConfig(
            steps=(4.0, 8.0), step_s=30, warmup_s=60, tx_timeout_s=30
        )
        self.assertEqual(awsb.load_seconds(load), 60 + 3 * 30 + 30)

    def test_worst_uses_ready_timeout_and_collect_max(self):
        cfg = awsb.RunConfig(tag="x", ready_timeout_s=900.0)
        phases = awsb.phase_seconds(cfg)
        self.assertEqual(phases["ready"], (awsb.READY_EXPECTED_S, 900.0))
        self.assertEqual(
            phases["collect"], (awsb.COLLECT_EXPECTED_S, awsb.COLLECT_MAX_S)
        )


# REQ:awsbench-cost-bound
class EstimateCostTest(unittest.TestCase):
    def setUp(self):
        self.cfg = awsb.RunConfig(
            tag="x",
            nodes=2,
            pg_iops=6000,
            pg_mbps=500,
            load=netbench.BenchConfig(submit_nodes=1),
        )
        self.hosts = awsb.plan_hosts(self.cfg)
        self.prices = two_node_prices()
        self.minor = awsb.MINOR_PRICES["eu-west-1"]

    def test_matches_hand_computed_totals(self):
        # Literal dollar values for this 2-node config, hand-computed independently of
        # estimate_cost's formula, so a formula regression trips this test.
        estimate = awsb.estimate_cost(self.hosts, self.cfg, self.prices, self.minor)
        self.assertEqual(estimate["expected_s"], 1650.0)
        self.assertEqual(estimate["ttl_s"], 3270.0)
        self.assertAlmostEqual(estimate["expected_usd"], 0.8683328695776255, places=6)
        self.assertAlmostEqual(estimate["bound_usd"], 1.7053451983447487, places=6)

    def test_bound_is_cost_at_ttl(self):
        estimate = awsb.estimate_cost(self.hosts, self.cfg, self.prices, self.minor)
        ratio = estimate["ttl_s"] / estimate["expected_s"]
        # every line scales with duration except egress, which is flat per host
        egress_line = next(
            line for line in estimate["lines"] if line["item"] == "egress"
        )
        scaled_expected = estimate["expected_usd"] - egress_line["usd"]
        self.assertAlmostEqual(
            estimate["bound_usd"],
            scaled_expected * ratio + egress_line["usd"],
            places=4,
        )


# EDGE:awsbench-unknown-region
class RegionMinorPricesTest(unittest.TestCase):
    def test_unknown_region_refuses(self):
        with self.assertRaises(awsb.Refused):
            awsb.region_minor_prices("ap-south-2")

    def test_known_region_ok(self):
        self.assertIn("gp3_gb_month_usd", awsb.region_minor_prices("eu-west-1"))


class FormatEstimateTest(unittest.TestCase):
    def test_contains_expected_and_bound(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        minor = awsb.region_minor_prices(cfg.region)
        estimate = awsb.estimate_cost(hosts, cfg, two_node_prices(), minor)
        text = awsb.format_estimate(estimate, cfg.max_usd)
        self.assertIn(f"expected ${estimate['expected_usd']:.2f}", text)
        self.assertIn(f"hard bound ${estimate['bound_usd']:.2f}", text)
        self.assertIn("limit $10.00", text)


class ConfirmTest(unittest.TestCase):
    def test_yes_flag_always_confirms(self):
        self.assertTrue(awsb.confirm("go?", yes=True, stdin_is_tty=False))

    def test_non_tty_without_yes_refuses(self):
        self.assertFalse(awsb.confirm("go?", yes=False, stdin_is_tty=False))

    def test_tty_reads_input(self):
        with unittest.mock.patch("builtins.input", return_value="y"):
            self.assertTrue(awsb.confirm("go?", yes=False, stdin_is_tty=True))
        with unittest.mock.patch("builtins.input", return_value="n"):
            self.assertFalse(awsb.confirm("go?", yes=False, stdin_is_tty=True))


# EDGE:awsbench-price-override
class FetchPricesTest(unittest.TestCase):
    def test_override_skips_fetch_entirely(self):
        with tempfile.TemporaryDirectory() as tmp:
            cfg = awsb.RunConfig(
                tag="x", nodes=2, price=("c8g.4xlarge=0.71", "c8g.2xlarge=0.355")
            )
            runner = FakeRunner({})
            prices = awsb.resolve_prices(
                runner, cfg, Path(tmp) / "prices.json", now=0.0
            )
            self.assertEqual(runner.calls, [])
            self.assertEqual(prices["instances"]["c8g.4xlarge"]["usd_hour"], 0.71)
            self.assertEqual(
                prices["instances"]["c8g.4xlarge"]["source"], "--price override"
            )

    def test_fresh_cache_skips_fetch(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            cache.write_text(
                json.dumps(
                    {"eu-west-1:c8g.4xlarge": {"usd_hour": 0.5, "fetched_at": 1000.0}}
                )
            )
            runner = FakeRunner({})
            prices = awsb.fetch_prices(
                runner,
                awsb.RunConfig(tag="x"),
                {"c8g.4xlarge"},
                cache,
                now=1000.0 + 3600,
            )
            self.assertEqual(runner.calls, [])
            self.assertEqual(prices["instances"]["c8g.4xlarge"]["usd_hour"], 0.5)

    def test_expired_cache_refetches(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            old = 1000.0
            cache.write_text(
                json.dumps(
                    {"eu-west-1:c8g.4xlarge": {"usd_hour": 0.5, "fetched_at": old}}
                )
            )
            now = old + awsb.PRICE_CACHE_TTL_S + 1
            runner = FakeRunner(
                {
                    (
                        "aws",
                        "--profile",
                        "timeboost-dev",
                        "pricing",
                        "get-products",
                    ): price_response(0.9)
                }
            )
            prices = awsb.fetch_prices(
                runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"}, cache, now=now
            )
            self.assertTrue(
                runner.ran(
                    "aws", "--profile", "timeboost-dev", "pricing", "get-products"
                )
            )
            self.assertEqual(prices["instances"]["c8g.4xlarge"]["usd_hour"], 0.9)
            self.assertEqual(
                json.loads(cache.read_text())["eu-west-1:c8g.4xlarge"]["usd_hour"], 0.9
            )

    def test_api_failure_without_override_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            runner = FakeRunner(
                {
                    (
                        "aws",
                        "--profile",
                        "timeboost-dev",
                        "pricing",
                        "get-products",
                    ): completed(returncode=1, stderr="AccessDenied")
                }
            )
            with self.assertRaises(awsb.Refused):
                awsb.fetch_prices(
                    runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"}, cache, now=0.0
                )


# REQ:awsbench-render-offline / REQ:awsbench-budget-refusal / EDGE:awsbench-name-collision
class CmdPlanTest(unittest.TestCase):
    def test_offline_renders_manifest_and_peers(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "2",
                    "--offline",
                    "--name",
                    "run1",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({})
            code = awsb.cmd_plan(args, run=runner)
            self.assertEqual(code, awsb.EXIT_OK)
            self.assertEqual(runner.calls, [])
            manifest = json.loads(
                (Path(out_root) / "run1" / "manifest.json").read_text()
            )
            self.assertEqual(manifest["phase"], "planned")
            self.assertEqual(manifest["peers"], {"node0": [], "node1": []})
            self.assertEqual(len(manifest["hosts"]), 3)
            self.assertTrue((Path(out_root) / "run1" / "driver.log").exists())
            self.assertTrue((Path(out_root) / "run1" / "events.jsonl").exists())

    def test_budget_refusal_makes_no_tofu_apply(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "5",
                    "--max-usd",
                    "0.01",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({STS_CALL: sts_response("027574771971")})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertFalse(runner.ran("tofu"))

    # REQ:awsbench-render-offline
    def test_offline_without_price_refuses(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                ["--tag", "x", "--nodes", "2", "--offline", "--out-root", out_root]
            )
            runner = FakeRunner({})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertEqual(runner.calls, [])

    def test_name_collision_refuses(self):
        with tempfile.TemporaryDirectory() as out_root:
            common = [
                "--tag",
                "x",
                "--name",
                "dup",
                "--out-root",
                out_root,
                "--price",
                "c8g.4xlarge=0.71",
                "--price",
                "c8g.2xlarge=0.355",
            ]
            first = awsb.cmd_plan(
                parse_plan_args(common),
                run=FakeRunner({STS_CALL: sts_response("027574771971")}),
            )
            self.assertEqual(first, awsb.EXIT_OK)
            second_runner = FakeRunner({})
            second = cmd_plan_exit(parse_plan_args(common), run=second_runner)
            self.assertEqual(second, awsb.EXIT_REFUSED)
            # the name collision is caught before any AWS call, including the account guard
            self.assertEqual(second_runner.calls, [])

    # EDGE:awsbench-name-validation
    def test_name_path_escape_refuses(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "2",
                    "--name",
                    "../escape",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertEqual(runner.calls, [])
            self.assertFalse((Path(out_root) / ".." / "escape").resolve().exists())

    def test_default_name_is_valid(self):
        self.assertRegex(awsb.default_run_name(), awsb.NAME_RE.pattern)

    # REQ:awsbench-account-guard
    def test_online_account_mismatch_makes_exactly_one_call(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "2",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({STS_CALL: sts_response("999999999999")})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertEqual(len(runner.calls), 1)


def two_node_hosts() -> list["awsb.HostSpec"]:
    return awsb.plan_hosts(
        awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    )


# REQ:awsbench-account-guard
class CallerAccountTest(unittest.TestCase):
    def test_mismatch_refuses_after_exactly_one_call(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "sts",
                    "get-caller-identity",
                ): completed(stdout=json.dumps({"Account": "999999999999"}))
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.caller_account(runner, awsb.RunConfig(tag="x"))
        self.assertEqual(len(runner.calls), 1)

    def test_match_returns_account(self):
        cfg = awsb.RunConfig(tag="x")
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "sts",
                    "get-caller-identity",
                ): completed(stdout=json.dumps({"Account": cfg.account}))
            }
        )
        self.assertEqual(awsb.caller_account(runner, cfg), cfg.account)


class DefaultVpcTest(unittest.TestCase):
    def test_no_default_vpc_refuses(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpcs",
                ): completed(stdout=json.dumps({"Vpcs": []}))
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.default_vpc(runner, awsb.RunConfig(tag="x"))

    def test_dns_hostnames_disabled_refuses(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpcs",
                ): completed(stdout=json.dumps({"Vpcs": [{"VpcId": "vpc-1"}]})),
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpc-attribute",
                ): completed(
                    stdout=json.dumps({"EnableDnsHostnames": {"Value": False}})
                ),
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.default_vpc(runner, awsb.RunConfig(tag="x"))

    def test_ok_returns_vpc_id(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpcs",
                ): completed(stdout=json.dumps({"Vpcs": [{"VpcId": "vpc-1"}]})),
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpc-attribute",
                ): completed(
                    stdout=json.dumps({"EnableDnsHostnames": {"Value": True}})
                ),
            }
        )
        self.assertEqual(awsb.default_vpc(runner, awsb.RunConfig(tag="x")), "vpc-1")


class CapableAzTest(unittest.TestCase):
    def _offerings(self, azs: list[str]) -> subprocess.CompletedProcess:
        return completed(
            stdout=json.dumps(
                {"InstanceTypeOfferings": [{"Location": az} for az in azs]}
            )
        )

    def test_intersects_and_picks_lowest_az(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-instance-type-offerings",
                ): self._offerings(["eu-west-1b", "eu-west-1a"])
            }
        )
        az = awsb.capable_az(runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"})
        self.assertEqual(az, "eu-west-1a")

    def test_no_capable_az_refuses(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-instance-type-offerings",
                ): self._offerings([])
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.capable_az(runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"})


class VcpuHeadroomTest(unittest.TestCase):
    def _quota_runner(self, quota):
        return FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "service-quotas",
                    "get-service-quota",
                ): completed(stdout=json.dumps({"Quota": {"Value": quota}}))
            }
        )

    def _instance_types(self, vcpus_by_type):
        return {t: {"Type": t, "VCpus": v} for t, v in vcpus_by_type.items()}

    def test_within_quota_ok(self):
        instance_types = self._instance_types({"c8g.4xlarge": 16, "c8g.2xlarge": 8})
        needed, in_use, quota = awsb.vcpu_headroom(
            self._quota_runner(256.0),
            awsb.RunConfig(tag="x"),
            two_node_hosts(),
            running=[],
            instance_types=instance_types,
        )
        self.assertEqual((needed, in_use, quota), (16 + 8 + 16, 0, 256.0))

    def test_over_quota_refuses(self):
        instance_types = self._instance_types({"c8g.4xlarge": 16, "c8g.2xlarge": 8})
        with self.assertRaises(awsb.Refused):
            awsb.vcpu_headroom(
                self._quota_runner(10.0),
                awsb.RunConfig(tag="x"),
                two_node_hosts(),
                running=[],
                instance_types=instance_types,
            )

    def test_repeated_running_type_counted_per_instance(self):
        # Three running c8g.4xlarge must count as 3x16 vCPUs, not once.
        instance_types = self._instance_types({"c8g.4xlarge": 16, "c8g.2xlarge": 8})
        running = ["c8g.4xlarge", "c8g.4xlarge", "c8g.4xlarge"]
        _needed, in_use, _quota = awsb.vcpu_headroom(
            self._quota_runner(256.0),
            awsb.RunConfig(tag="x"),
            two_node_hosts(),
            running=running,
            instance_types=instance_types,
        )
        self.assertEqual(in_use, 48)


class PreflightTest(unittest.TestCase):
    """Runs `preflight` with a fake `which` (no real tools needed) and a stubbed
    `resolve_image`, and asserts `describe-instance-types` is called exactly once:
    `resolve_ami_arch` and `vcpu_headroom` used to each fetch it separately."""

    def test_describe_instance_types_called_once(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        responses = {
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "sts",
                "get-caller-identity",
            ): completed(stdout=json.dumps({"Account": cfg.account})),
            ("aws", "--profile", "timeboost-dev", "ec2", "describe-vpcs"): completed(
                stdout=json.dumps({"Vpcs": [{"VpcId": "vpc-1"}]})
            ),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-vpc-attribute",
            ): completed(stdout=json.dumps({"EnableDnsHostnames": {"Value": True}})),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instance-type-offerings",
            ): completed(
                stdout=json.dumps(
                    {"InstanceTypeOfferings": [{"Location": "eu-west-1a"}]}
                )
            ),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instances",
            ): completed(stdout=json.dumps([])),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instance-types",
            ): completed(
                stdout=json.dumps(
                    [
                        {"Type": "c8g.4xlarge", "VCpus": 16, "Archs": ["arm64"]},
                        {"Type": "c8g.2xlarge", "VCpus": 8, "Archs": ["arm64"]},
                    ]
                )
            ),
            ("aws", "--profile", "timeboost-dev", "service-quotas"): completed(
                stdout=json.dumps({"Quota": {"Value": 256.0}})
            ),
            ("aws", "--profile", "timeboost-dev", "ec2", "describe-images"): completed(
                stdout="ami-0abc\n"
            ),
            ("git", "status"): completed(stdout=""),
        }
        runner = FakeRunner(responses)
        fake_image = {
            "ref": "x",
            "digest": "sha256:" + "a" * 64,
            "revision": None,
            "platforms": ["linux/arm64"],
        }
        with unittest.mock.patch.object(awsb, "resolve_image", return_value=fake_image):
            result = awsb.preflight(runner, cfg, hosts, which=lambda name: "/usr/bin/x")
        describe_calls = [
            c
            for c in runner.calls
            if tuple(c[:5])
            == ("aws", "--profile", "timeboost-dev", "ec2", "describe-instance-types")
        ]
        self.assertEqual(len(describe_calls), 1)
        self.assertEqual(result["az"], "eu-west-1a")
        self.assertEqual(result["ami_id"], "ami-0abc")


class CheckGitCleanTest(unittest.TestCase):
    def test_clean_tree_returns_none(self):
        runner = FakeRunner({("git", "status"): completed(stdout="")})
        self.assertIsNone(awsb.check_git_clean(runner, allow_dirty=False))

    def test_dirty_without_allow_dirty_refuses(self):
        runner = FakeRunner({("git", "status"): completed(stdout=" M file.py\n")})
        with self.assertRaises(awsb.Refused):
            awsb.check_git_clean(runner, allow_dirty=False)

    def test_dirty_with_allow_dirty_returns_diff(self):
        runner = FakeRunner(
            {
                ("git", "status"): completed(stdout=" M file.py\n"),
                ("git", "diff"): completed(stdout="--- a/file.py\n+++ b/file.py\n"),
            }
        )
        diff = awsb.check_git_clean(runner, allow_dirty=True)
        self.assertIn("file.py", diff)


class ToolsOnPathTest(unittest.TestCase):
    def test_missing_tool_refuses(self):
        with self.assertRaises(awsb.Refused):
            awsb.tools_on_path(("definitely-not-a-real-tool-xyz",))

    def test_present_tools_ok(self):
        awsb.tools_on_path(("python3",))

    def test_injectable_which_needs_no_real_tools(self):
        awsb.tools_on_path(("tofu", "aws"), which=lambda name: f"/usr/bin/{name}")
        with self.assertRaises(awsb.Refused):
            awsb.tools_on_path(("tofu",), which=lambda name: None)


class RenderTfvarsTest(unittest.TestCase):
    def test_shape_and_host_count(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        run_dir = Path("/tmp/aws-bench/run1")
        tfvars = awsb.render_tfvars(
            cfg,
            run_dir,
            "run1",
            "alice",
            hosts,
            ssh_pub="ssh-ed25519 AAAA test@example.com",
            operator_cidr="203.0.113.5/32",
            expires_at="2026-01-01T00:00:00Z",
            git_rev="abc1234",
            az="eu-west-1a",
            ami_id="ami-0abc",
        )
        self.assertEqual(tfvars["name"], "run1")
        self.assertEqual(tfvars["owner"], "alice")
        self.assertEqual(tfvars["account_id"], cfg.account)
        self.assertEqual(tfvars["profile"], cfg.profile)
        self.assertEqual(tfvars["az"], "eu-west-1a")
        self.assertEqual(tfvars["ami_id"], "ami-0abc")
        self.assertEqual(tfvars["operator_cidr"], "203.0.113.5/32")
        self.assertFalse(tfvars["offline"])
        self.assertEqual(len(tfvars["hosts"]), 3)
        self.assertEqual(
            tfvars["hosts"]["node0"]["user_data_path"],
            str(run_dir / "hosts" / "node0" / "user-data.sh"),
        )

    def test_offline_flag_passes_through(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, offline=True, load=netbench.BenchConfig(submit_nodes=1)
        )
        tfvars = awsb.render_tfvars(
            cfg,
            Path("/tmp/aws-bench/run1"),
            "run1",
            "alice",
            awsb.plan_hosts(cfg),
            ssh_pub="k",
            operator_cidr="203.0.113.5/32",
            expires_at="2026-01-01T00:00:00Z",
            git_rev="abc1234",
            az="",
            ami_id="",
        )
        self.assertTrue(tfvars["offline"])


class ClassifyTfErrorTest(unittest.TestCase):
    def test_vcpu_limit(self):
        self.assertIn(
            "VcpuLimitExceeded", awsb.classify_tf_error("... VcpuLimitExceeded ...")
        )
        self.assertIn("quota-code", awsb.classify_tf_error("... VcpuLimitExceeded ..."))

    def test_insufficient_capacity(self):
        self.assertIn(
            "--az", awsb.classify_tf_error("Error: InsufficientInstanceCapacity")
        )

    def test_unauthorized(self):
        self.assertIn(
            "IAM", awsb.classify_tf_error("Error: UnauthorizedOperation: ...")
        )

    def test_unclassified_keeps_last_line(self):
        result = awsb.classify_tf_error("line one\nline two\nsome other tofu error\n")
        self.assertIn("some other tofu error", result)


class TerraformClassTest(unittest.TestCase):
    def test_init_plan_apply_destroy(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        runner = FakeRunner(
            {
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "init",
                ): completed(),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "plan",
                ): completed(stdout="plan ok"),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "apply",
                ): completed(),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "output",
                ): completed(stdout=json.dumps({"az": {"value": "eu-west-1a"}})),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "destroy",
                ): completed(),
            }
        )
        tf = awsb.Terraform(runner, "tofu", tf_dir, env={})
        tf.init()
        tf.plan()
        tf.apply()
        self.assertEqual(tf.output(), {"az": {"value": "eu-west-1a"}})
        tf.destroy()
        self.assertTrue(runner.ran("tofu", f"-chdir={tf_dir}", "apply"))

    def test_apply_failure_is_classified(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        runner = FakeRunner(
            {
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "apply",
                ): completed(returncode=1, stderr="Error: VcpuLimitExceeded: ...")
            }
        )
        tf = awsb.Terraform(runner, "tofu", tf_dir, env={})
        with self.assertRaises(awsb.TfFailed) as ctx:
            tf.apply()
        self.assertEqual(ctx.exception.stage, "apply")
        self.assertIn("VcpuLimitExceeded", str(ctx.exception))

    def test_destroy_failure_raises_tf_failed(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        runner = FakeRunner(
            {
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "destroy",
                ): completed(returncode=1, stderr="boom")
            }
        )
        tf = awsb.Terraform(runner, "tofu", tf_dir, env={})
        with self.assertRaises(awsb.TfFailed) as ctx:
            tf.destroy()
        self.assertEqual(ctx.exception.stage, "destroy")

    def test_env_is_exported(self):
        # A key that never varies between tests: a per-run value like AWS_REGION would leak
        # into every test that runs afterward, since os.environ is process-global.
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        with unittest.mock.patch.dict(os.environ, {}, clear=False):
            awsb.Terraform(
                FakeRunner({}),
                "tofu",
                tf_dir,
                env={"TF_PLUGIN_CACHE_DIR": "/tmp/plugins"},
            )
            self.assertEqual(os.environ["TF_PLUGIN_CACHE_DIR"], "/tmp/plugins")


SINGLE_MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json"


class FakeRegistry(ThreadingHTTPServer):
    """A minimal OCI/Docker registry: one repository and tag, anonymous token challenge.
    `platforms` is a list of `(os, architecture)` pairs for the index; a `tag` of `"missing"`
    makes the manifest request 404, `deny_token` makes the token endpoint 401 (a private
    image), `deny_manifest_status` makes the manifest request fail with that status without
    ever offering a token challenge, and `index=False` serves a single manifest at the tag
    instead of a multi-platform index."""

    def __init__(
        self,
        repository: str,
        tag: str,
        platforms: list[tuple[str, str]],
        revision: str | None = None,
        deny_token: bool = False,
        deny_manifest_status: int | None = None,
        index: bool = True,
    ):
        super().__init__(("127.0.0.1", 0), FakeRegistryHandler)
        self.repository = repository
        self.tag = tag
        self.platforms = platforms
        self.revision = revision
        self.deny_token = deny_token
        self.deny_manifest_status = deny_manifest_status
        self.index = index
        self.digests = {p: f"sha256:{i:064d}" for i, p in enumerate(platforms)}
        self.config_digest = "sha256:" + "c" * 64

    @property
    def host(self) -> str:
        return f"127.0.0.1:{self.server_address[1]}"

    @property
    def ref(self) -> str:
        return f"{self.host}/{self.repository}:{self.tag}"


class FakeRegistryHandler(BaseHTTPRequestHandler):
    server: FakeRegistry

    def log_message(self, format, *args):
        pass

    def _reply_json(
        self, status: int, obj: dict, content_type: str = "application/json"
    ):
        body = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        server = self.server
        if self.path.startswith("/token"):
            if server.deny_token:
                self.send_response(401)
                self.end_headers()
                return
            self._reply_json(200, {"token": "faketoken"})
            return

        manifest_prefix = f"/v2/{server.repository}/manifests/"
        blob_prefix = f"/v2/{server.repository}/blobs/"

        if self.path.startswith(manifest_prefix):
            if server.deny_manifest_status is not None:
                self.send_response(server.deny_manifest_status)
                self.end_headers()
                return
            if self.headers.get("Authorization") != "Bearer faketoken":
                self.send_response(401)
                self.send_header(
                    "WWW-Authenticate",
                    f'Bearer realm="http://{server.host}/token",service="fake",'
                    f'scope="repository:{server.repository}:pull"',
                )
                self.end_headers()
                return
            ref = self.path[len(manifest_prefix) :]
            if ref == server.tag:
                if server.tag == "missing":
                    self.send_response(404)
                    self.end_headers()
                    return
                if not server.index:
                    self._reply_json(
                        200,
                        {
                            "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                            "config": {"digest": server.config_digest},
                        },
                    )
                    return
                manifests = [
                    {
                        "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                        "digest": server.digests[p],
                        "platform": {"os": p[0], "architecture": p[1]},
                    }
                    for p in server.platforms
                ]
                self._reply_json(
                    200,
                    {
                        "mediaType": "application/vnd.oci.image.index.v1+json",
                        "manifests": manifests,
                    },
                    content_type="application/vnd.oci.image.index.v1+json",
                )
                return
            if ref == server.config_digest:
                self.send_response(404)
                self.end_headers()
                return
            matching = [p for p, digest in server.digests.items() if digest == ref]
            if not matching:
                self.send_response(404)
                self.end_headers()
                return
            self._reply_json(
                200,
                {
                    "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                    "config": {"digest": server.config_digest},
                },
            )
            return

        if self.path.startswith(blob_prefix):
            digest = self.path[len(blob_prefix) :]
            if digest != server.config_digest:
                self.send_response(404)
                self.end_headers()
                return
            labels = (
                {"org.opencontainers.image.revision": server.revision}
                if server.revision is not None
                else {}
            )
            self._reply_json(
                200,
                {"architecture": "arm64", "os": "linux", "config": {"Labels": labels}},
            )
            return

        self.send_response(404)
        self.end_headers()


# REQ:awsbench-image-check
class ResolveImageTest(unittest.TestCase):
    def _serve(self, **kwargs) -> FakeRegistry:
        server = FakeRegistry(**kwargs)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        # addCleanup is LIFO: register in reverse of the intended shutdown -> join ->
        # server_close order, since join() needs shutdown() to have stopped serve_forever.
        self.addCleanup(server.server_close)
        self.addCleanup(thread.join)
        self.addCleanup(server.shutdown)
        return server

    def test_resolves_digest_platforms_and_revision(self):
        server = self._serve(
            repository="test/image",
            tag="v1",
            platforms=[("linux", "amd64"), ("linux", "arm64")],
            revision="abc1234",
        )
        info = awsb.resolve_image(server.ref)
        self.assertEqual(info["digest"], server.digests[("linux", "arm64")])
        self.assertCountEqual(info["platforms"], ["linux/amd64", "linux/arm64"])
        self.assertEqual(info["revision"], "abc1234")
        self.assertEqual(info["ref"], server.ref)

    def test_no_revision_label_is_none(self):
        server = self._serve(repository="x", tag="v1", platforms=[("linux", "arm64")])
        info = awsb.resolve_image(server.ref)
        self.assertIsNone(info["revision"])

    def test_single_manifest_without_index(self):
        server = self._serve(
            repository="x", tag="v1", platforms=[("linux", "arm64")], index=False
        )
        info = awsb.resolve_image(server.ref)
        self.assertEqual(info["platforms"], ["linux/arm64"])
        self.assertTrue(info["digest"].startswith("sha256:"))

    def test_attestation_platform_ignored(self):
        # buildx publishes an extra unknown/unknown manifest for SBOM/provenance attestations.
        server = self._serve(
            repository="x",
            tag="v1",
            platforms=[("unknown", "unknown"), ("linux", "arm64")],
        )
        info = awsb.resolve_image(server.ref)
        self.assertEqual(info["platforms"], ["linux/arm64"])

    # EDGE:awsbench-image-no-arm64
    def test_missing_arm64_refuses(self):
        server = self._serve(repository="x", tag="v1", platforms=[("linux", "amd64")])
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("no linux/arm64 platform", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    def test_missing_tag_refuses(self):
        server = self._serve(
            repository="x", tag="missing", platforms=[("linux", "arm64")]
        )
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("image not found", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    # EDGE:awsbench-image-private
    def test_private_image_denied_token_refuses(self):
        server = self._serve(
            repository="x", tag="v1", platforms=[("linux", "arm64")], deny_token=True
        )
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("token request", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    def test_private_image_403_refuses(self):
        server = self._serve(
            repository="x",
            tag="v1",
            platforms=[("linux", "arm64")],
            deny_manifest_status=403,
        )
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("private or inaccessible", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))


class ParseRefTest(unittest.TestCase):
    def test_docker_hub_official_image(self):
        self.assertEqual(
            awsb.parse_ref("postgres:16"),
            ("registry-1.docker.io", "library/postgres", "16"),
        )

    def test_ghcr_multi_segment(self):
        self.assertEqual(
            awsb.parse_ref("ghcr.io/espressosystems/espresso-network/espresso-node:x"),
            ("ghcr.io", "espressosystems/espresso-network/espresso-node", "x"),
        )

    def test_no_tag_defaults_to_latest(self):
        self.assertEqual(
            awsb.parse_ref("ghcr.io/foundry-rs/foundry"),
            ("ghcr.io", "foundry-rs/foundry", "latest"),
        )

    def test_docker_io_alias_resolves_to_registry_host(self):
        # SUPPORT_IMAGES["postgres"]; a literal docker.io host must not be sent to the
        # registry as-is (https://docker.io/v2/... 302s to www.docker.com).
        self.assertEqual(
            awsb.parse_ref("docker.io/library/postgres:16"),
            ("registry-1.docker.io", "library/postgres", "16"),
        )
        self.assertEqual(
            awsb.parse_ref("index.docker.io/library/postgres:16"),
            ("registry-1.docker.io", "library/postgres", "16"),
        )


TOFU = shutil.which("tofu")


@unittest.skipUnless(TOFU, "tofu/opentofu not on PATH")
class TofuValidateTest(unittest.TestCase):
    """TEST:awsbench-tofu-validate-ok: validates the module and plans it with offline=true, so
    it needs no real AWS credentials; skips entirely when tofu is absent from the environment."""

    def test_validate_and_offline_plan(self):
        assert TOFU is not None  # narrows the type; the class is skipped otherwise
        terraform_src = Path(__file__).parent / "aws" / "terraform"
        with tempfile.TemporaryDirectory() as tmp:
            terraform_dir = Path(tmp) / "terraform"
            shutil.copytree(terraform_src, terraform_dir)
            # offline=true still evaluates for_each = var.hosts (main.tf), so the user-data
            # file must exist for `file()` to succeed at plan time.
            user_data_path = terraform_dir / "ctl-user-data.sh"
            user_data_path.write_text("#!/bin/sh\necho ok\n")
            tfvars = {
                "name": "test-run",
                "owner": "tester",
                "git_rev": "abc1234",
                "account_id": "000000000000",
                "region": "eu-west-1",
                "profile": "test",
                "az": "",
                "ami_id": "",
                "ssh_public_key": "ssh-ed25519 AAAAtest test@example.com",
                "operator_cidr": "203.0.113.5/32",
                "expires_at": "2026-01-01T00:00:00Z",
                "offline": True,
                "hosts": {
                    "ctl": {
                        "role": "ctl",
                        "instance_type": "c8g.2xlarge",
                        "root_gb": 40,
                        "root_iops": 3000,
                        "root_mbps": 125,
                        "user_data_path": str(user_data_path),
                    }
                },
            }
            (terraform_dir / "terraform.tfvars.json").write_text(json.dumps(tfvars))
            env = {
                **os.environ,
                "AWS_ACCESS_KEY_ID": "test",
                "AWS_SECRET_ACCESS_KEY": "test",
                "AWS_REGION": "eu-west-1",
            }
            init = subprocess.run(
                [TOFU, f"-chdir={terraform_dir}", "init", "-input=false"],
                capture_output=True,
                text=True,
                env=env,
                check=False,
            )
            self.assertEqual(init.returncode, 0, init.stderr)
            validate = subprocess.run(
                [TOFU, f"-chdir={terraform_dir}", "validate"],
                capture_output=True,
                text=True,
                env=env,
                check=False,
            )
            self.assertEqual(validate.returncode, 0, validate.stderr)
            plan = subprocess.run(
                [
                    TOFU,
                    f"-chdir={terraform_dir}",
                    "plan",
                    "-input=false",
                    "-refresh=false",
                ],
                capture_output=True,
                text=True,
                env=env,
                check=False,
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)


if __name__ == "__main__":
    unittest.main()
