"""Tests for `aws-bench`: cost estimate, budget refusal, and `plan --offline` rendering.

Side effects go through a `FakeRunner` that maps an argv prefix to a canned
`subprocess.CompletedProcess`, and records every call, so a refused plan can be shown to have
made no `aws` call at all.

    just py::test
"""

import importlib.util
import json
import subprocess
import tempfile
import unittest
import unittest.mock
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

    def test_budget_refusal_makes_no_aws_call(self):
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
            runner = FakeRunner({})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertEqual(runner.calls, [])

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
            first = awsb.cmd_plan(parse_plan_args(common), run=FakeRunner({}))
            self.assertEqual(first, awsb.EXIT_OK)
            second = cmd_plan_exit(parse_plan_args(common), run=FakeRunner({}))
            self.assertEqual(second, awsb.EXIT_REFUSED)

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


if __name__ == "__main__":
    unittest.main()
