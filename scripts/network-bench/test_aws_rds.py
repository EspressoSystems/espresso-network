"""Tests for the rds and pg volume query stores of `aws-bench`, and the sweep of their resources."""

import json
import unittest
import unittest.mock
from collections.abc import Iterator
from datetime import UTC, datetime, timedelta
from pathlib import Path

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    EXPIRES_LATER,
    EXPIRES_PAST,
    LIST_ROLES,
    NOW,
    RDS_CREATED,
    RDS_OUTPUT,
    ROLE_ARN,
    STATUS_DESCRIBE,
    VOLUME_ID,
    FakeClock,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    RdsHarness,
    RdsRunner,
    aws_manifest,
    aws_verbs,
    awsb,
    completed,
    db_instance,
    fake_preflight,
    instance_row,
    isolated_env,
    mapping,
    mode,
    rds_fleet_mappings,
    rds_spec,
    rds_tag_runner,
    resource_arn,
    run_cmd,
    ssh_calls,
    two_node_hosts_info,
    valid_result,
    write_collected_run,
)


@pytest.fixture
def case() -> Iterator[unittest.TestCase]:
    """The cleanup and assert target the `fakes` harnesses take."""
    case = unittest.TestCase()
    yield case
    case.doCleanups()


@pytest.fixture
def harness(isolated: Path, case: unittest.TestCase) -> RdsHarness:
    return RdsHarness(case)


def failing(runner: FakeRunner, pattern: str, stderr: str) -> None:
    runner.respond(pattern, lambda _: completed(returncode=254, stderr=stderr))


@pytest.mark.parametrize(
    ("flag", "value", "pattern"),
    [("--pg-iops", "6000", "--pg-iops 12000"), ("--pg-mbps", "250", "--pg-mbps 500")],
)
def test_other_iops_or_throughput_are_refused(harness, flag, value, pattern):
    runner = FakeRunner()
    with pytest.raises(awsb.Refused, match=pattern):
        harness.up(runner, flag, value)
    assert runner.calls == []


def orderable(**overrides) -> dict:
    option = {
        "Engine": "postgres",
        "EngineVersion": "18.2",
        "DBInstanceClass": "db.m8g.4xlarge",
        "StorageType": "gp3",
        "AvailabilityZones": [{"Name": "eu-west-1a"}, {"Name": "eu-west-1b"}],
        "SupportsPerformanceInsights": True,
        "MinStorageSize": 400,
        "MaxStorageSize": 65536,
        "MinIopsPerDbInstance": 12000,
        "MaxIopsPerDbInstance": 64000,
        "MinStorageThroughputPerDbInstance": 500,
        "MaxStorageThroughputPerDbInstance": 4000,
    }
    return {**option, **overrides}


def orderable_runner(*options: dict) -> FakeRunner:
    response = completed(stdout=json.dumps({"OrderableDBInstanceOptions": options}))
    return FakeRunner({("aws", "--profile", "timeboost-dev", "rds"): response})


RDS_CFG = awsb.RunConfig(tag="x", db_modes=("rds",))


def test_a_major_resolves_to_the_highest_minor_that_fits():
    runner = orderable_runner(
        orderable(EngineVersion="18.1"),
        orderable(EngineVersion="18.10"),
        orderable(EngineVersion="18.9"),
        orderable(EngineVersion="17.6"),
        orderable(EngineVersion="18.11", StorageType="io2"),
        orderable(EngineVersion="18.12", AvailabilityZones=[{"Name": "eu-west-1c"}]),
    )
    assert awsb.rds_orderable(runner, RDS_CFG, "eu-west-1b") == "18.10"


@pytest.mark.parametrize(
    ("runner", "pattern"),
    [
        (
            orderable_runner(orderable(AvailabilityZones=[{"Name": "eu-west-1a"}])),
            "db.m8g.4xlarge cannot run postgres 18",
        ),
        (
            FakeRunner({("aws",): completed(returncode=254, stderr="AccessDenied")}),
            "AccessDenied",
        ),
    ],
    ids=["unorderable", "failed-call"],
)
def test_an_unorderable_rds_is_refused(runner, pattern):
    with pytest.raises(awsb.Refused, match=pattern):
        awsb.rds_orderable(runner, RDS_CFG, "eu-west-1b")


def test_the_fleet_bound_bills_rds_until_its_delete_finishes():
    load = netbench.BenchConfig(submit_nodes=1)
    cfg = awsb.RunConfig(tag="x", nodes=2, ttl_min="150", db_modes=("rds",), load=load)
    hosts = awsb.plan_hosts(cfg)
    ttl_s = 150 * 60.0
    without = awsb.cost_estimate(hosts, cfg, None, ttl_s, ttl_s)
    rds = rds_spec()
    with_rds = awsb.cost_estimate(hosts, cfg, rds, ttl_s, ttl_s)
    expected = sum(line["usd"] for line in awsb.rds_cost_lines(rds, ttl_s))
    assert with_rds["expected_usd"] - without["expected_usd"] == pytest.approx(expected)
    bound_lines = awsb.rds_cost_lines(rds, ttl_s + awsb.RDS_DELETE_S)
    bound = sum(line["usd"] for line in bound_lines)
    assert with_rds["bound_usd"] - without["bound_usd"] == pytest.approx(bound)
    assert awsb.estimate_rate(with_rds) > (
        awsb.estimate_rate(without) + awsb.PRICES["db.m8g.4xlarge"]
    )


def test_a_run_needs_time_for_the_rds_delete(harness, case):
    harness.up_rds(case)
    manifest = harness.fleet()
    cfg = awsb.fleet_run_config(harness.run_args("--query-db", "rds"), manifest)
    worst_s = awsb.estimate_run(manifest, cfg)["worst_s"]
    floor = worst_s + awsb.NOLOGIN_LEAD_S + awsb.DESTROY_S + awsb.RDS_DELETE_S
    expires = datetime.fromisoformat(manifest["expires_at"])
    with pytest.raises(awsb.Refused, match="up a new fleet"):
        awsb.check_run_allowed(manifest, cfg, expires - timedelta(seconds=floor - 60))
    awsb.check_run_allowed(manifest, cfg, expires - timedelta(seconds=floor + 60))


# TEST:querydb-settings-parity-ok
def test_the_parameter_group_renders_from_the_container_settings(harness, case):
    harness.up_rds(case)
    parameters = harness.tfvars()["rds"]["parameters"]
    for key, (_, setting) in awsb.PG_TUNING.items():
        assert parameters[key] == setting, key
    assert parameters["shared_buffers"] == "1048576"
    assert parameters["shared_preload_libraries"] == "pg_stat_statements"
    assert parameters["log_min_duration_statement"] == "200"
    assert parameters["pg_stat_statements.track"] == "all"
    assert "ssl" not in parameters
    assert "rds.force_ssl" not in parameters


def test_the_password_is_generated_private_and_rds_safe(harness, case):
    harness.up_rds(case)
    password = harness.tfvars()["rds_password"]
    assert len(password) >= 24
    assert password.replace("_", "").replace("-", "").isalnum()
    terraform = harness.fleet_dir / "terraform"
    assert mode(terraform) == 0o700
    assert mode(terraform / "terraform.tfvars.json") == 0o600
    assert password not in (harness.fleet_dir / "fleet.json").read_text()
    assert password not in harness.driver_log()


def test_the_delete_time_follows_the_confirmed_expiry(harness, case):
    harness.up_rds(case)
    delete_at = datetime.fromisoformat(harness.tfvars()["rds"]["delete_at"] + "+00:00")
    expected = NOW + timedelta(minutes=180, seconds=-awsb.RDS_REAPER_LEAD_S)
    assert abs((delete_at - expected).total_seconds()) < 60


def test_a_single_shot_deletes_the_rds_before_its_hosts_end(tmp_path):
    """The tag expiry counts the provisioning time, the hosts' own timers do not."""
    (tmp_path / "terraform").mkdir()
    netbench.write_json(
        tmp_path / "fleet.json", {"phase": "planned", "estimate": {"ttl_s": 3600.0}}
    )
    tfvars_path = tmp_path / "terraform" / "terraform.tfvars.json"
    netbench.write_json(tfvars_path, {"expires_at": "x", "rds": {"delete_at": "x"}})
    confirmed = datetime(2026, 9, 29, 16, 0, tzinfo=UTC)
    awsb.stamp_expiry(tmp_path, unittest.mock.Mock(), confirmed)
    written = json.loads(tfvars_path.read_text())
    assert written["expires_at"] == "2026-09-29T17:08:00Z"
    assert written["rds"]["delete_at"] == "2026-09-29T16:55:00"


def test_plan_renders_the_rds_variables_and_prices_them(isolated, case, monkeypatch):
    out = isolated_env(case, "planned")
    argv = ["plan", "--nodes", "2", "--tag", "x", "--db-modes", "colocated,rds"]
    args = awsb.parse_args(argv)
    args.argv = argv
    pre = {**fake_preflight(), "rds_engine_version": "18"}
    monkeypatch.setattr(awsb, "preflight", lambda *_: pre)
    system = FakeSystem(run=FakeRunner(states=[]))
    assert awsb.cmd_plan(args, system) == awsb.EXIT_OK
    tfvars = json.loads((out / "planned/terraform/terraform.tfvars.json").read_text())
    assert tfvars["rds"]["engine_version"] == "18"
    manifest = json.loads((out / "planned/fleet.json").read_text())
    assert manifest["db_modes"] == ["colocated", "rds"]
    items = {line["item"] for line in manifest["estimate"]["lines"]}
    assert {"rds instance db.m8g.4xlarge", "rds gp3 storage"} <= items


def rds_run_dir(harness: RdsHarness) -> tuple[Path, "awsb.RunConfig"]:
    run_dir = harness.run_dir("01-rds")
    run_dir.mkdir(parents=True)
    cfg = awsb.dataclasses.replace(
        awsb.config_from_manifest(harness.fleet()["config"]), query_db="rds"
    )
    return run_dir, cfg


def test_node0_env_points_at_the_endpoint(harness, case):
    harness.up_rds(case)
    rds = json.loads((harness.fleet_dir / "rds.json").read_text())
    run_dir, cfg = rds_run_dir(harness)
    awsb.render_host_files(run_dir, cfg, harness.fleet(), two_node_hosts_info())
    node0 = run_dir / "hosts/node0"
    env = dict(
        line.split("=", 1) for line in (node0 / "node.env").read_text().splitlines()
    )
    assert env["ESPRESSO_NODE_POSTGRES_HOST"] == RDS_OUTPUT["endpoint"]
    assert env["ESPRESSO_NODE_POSTGRES_USER"] == awsb.RDS_USER
    assert env["ESPRESSO_NODE_POSTGRES_PASSWORD"] == rds["password"]
    assert env["ESPRESSO_NODE_POSTGRES_DATABASE"] == "espresso"
    assert json.loads((node0 / "pg.json").read_text())["host"] == RDS_OUTPUT["endpoint"]
    assert mode(node0 / "pg.json") == 0o600
    assert mode(node0 / "node.env") == 0o600
    assert "--name postgres" not in (node0 / "start.sh").read_text()


def test_the_password_files_are_private_before_they_are_written(
    harness, case, monkeypatch
):
    harness.up_rds(case)
    run_dir, cfg = rds_run_dir(harness)
    modes: dict[str, int | None] = {}

    def spy(write):
        def recording(path: Path, *args, **kwargs):
            key = f"{path.parent.name}/{path.name}"
            modes[key] = mode(path) if path.exists() else None
            return write(path, *args, **kwargs)

        return recording

    monkeypatch.setattr(Path, "write_text", spy(Path.write_text))
    monkeypatch.setattr(netbench, "write_json", spy(netbench.write_json))
    awsb.render_host_files(run_dir, cfg, harness.fleet(), two_node_hosts_info())
    assert modes["node0/node.env"] == 0o600
    assert modes["node0/pg.json"] == 0o600


def test_up_records_the_instance_and_gates_node0_on_it(harness, case):
    runner = harness.up_rds(case)
    manifest = harness.fleet()
    assert manifest["phase"] == "idle"
    assert manifest["rds"] == {**RDS_OUTPUT, "created_at": RDS_CREATED}
    rds = json.loads((harness.fleet_dir / "rds.json").read_text())
    assert rds["endpoint"] == RDS_OUTPUT["endpoint"]
    assert rds["username"] == awsb.RDS_USER
    assert rds["password"] == harness.tfvars()["rds_password"]
    assert mode(harness.fleet_dir / "rds.json") == 0o600
    pg = json.loads((harness.fleet_dir / "hosts/node0/pg.json").read_text())
    assert pg["host"] == RDS_OUTPUT["endpoint"]
    assert runner.ran("rsync", f"{awsb.BENCH_DIR}/pg.json")
    commands = ssh_calls(runner)
    assert any("pg_isready" in c for c in commands)
    assert not any("docker start" in c for c in commands)
    assert not runner.ran("reboot-db-instance")
    assert not any(rds["password"] in " ".join(call) for call in runner.calls)


# TEST:querydb-rds-pending-reboot-ok
def test_pending_reboot_reboots_once_and_waits(harness, case):
    instances = [
        db_instance(applied="pending-reboot"),
        db_instance(state="rebooting", applied="pending-reboot"),
        db_instance(applied="in-sync"),
    ]
    runner = harness.up_rds(case, instances=instances)
    assert runner.count("reboot-db-instance") == 1
    assert harness.fleet()["phase"] == "idle"


@pytest.mark.parametrize(
    ("kwargs", "reboots"),
    [
        ({"instances": [db_instance(applied="pending-reboot")]}, 1),
        # TEST:querydb-rds-create-timeout-fails
        ({"apply": completed(returncode=1, stderr="Error: waiting for RDS")}, 0),
    ],
    ids=["parameters-stay-pending", "create-timeout"],
)
def test_an_rds_that_never_gets_ready_destroys_the_fleet(harness, kwargs, reboots):
    runner = RdsRunner([DONE_STATE], **kwargs)
    assert harness.up(runner) == awsb.EXIT_FAILED
    assert runner.ran("tofu", "destroy")
    assert runner.count("reboot-db-instance") == reboots


def test_a_null_rds_output_is_an_error():
    with pytest.raises(awsb.RemoteError, match="null"):
        awsb.parse_rds_output({"rds": {"value": None}})


def test_reset_drops_and_recreates_the_database_before_the_services(harness, case):
    runner = harness.up_rds(case)
    mark = len(runner.calls)
    assert harness.run(runner, "--query-db", "rds") == awsb.EXIT_OK
    calls = [" ".join(call) for call in runner.calls[mark:]]

    def first(*needle: str) -> int:
        return next(i for i, c in enumerate(calls) if all(n in c for n in needle))

    drop = first("ssh", "DROP DATABASE")
    assert first("ssh", "find /data/journal") < drop
    assert first("rsync", f"{awsb.BENCH_DIR}/pg.json") < drop
    assert drop < first("ssh", "docker start anvil")
    # TEST:querydb-rds-drop-force-ok
    statement = calls[drop]
    assert "DROP DATABASE IF EXISTS espresso WITH (FORCE)" in statement
    assert "CREATE DATABASE espresso" in statement
    assert "PGDATABASE=postgres" in statement
    assert "ON_ERROR_STOP=1" in statement
    assert harness.tfvars()["rds_password"] not in statement


# EDGE:querydb-mode-switch
def test_a_colocated_run_after_an_rds_run_starts_its_own_postgres(isolated, case):
    harness = RdsHarness(case, modes="colocated,rds")
    runner = harness.up_rds(case)
    harness.run(runner, "--query-db", "rds")
    mark = len(runner.calls)
    harness.run(runner, "--query-db", "colocated")
    commands = ssh_calls(runner, mark)
    assert not any("DROP DATABASE" in c for c in commands)
    assert any("docker start postgres" in c for c in commands)
    host_dir = harness.run_dir("02-colocated") / "hosts/node0"
    assert "--name postgres" in (host_dir / "start.sh").read_text()
    assert json.loads((host_dir / "pg.json").read_text())["host"] == "127.0.0.1"


def test_an_rds_run_needs_the_mode_in_the_fleet(isolated, case):
    harness = FleetHarness(case)
    runner = harness.up_fleet(case)
    with pytest.raises(awsb.Refused, match="--query-db rds was not provisioned"):
        harness.run(runner, "--query-db", "rds")


# EDGE:fleet-reset-fails
def test_a_failed_database_drop_leaves_the_fleet_dirty(harness, case):
    runner = harness.up_rds(case)
    failing(runner, "DROP DATABASE", "permission denied")
    assert harness.run(runner, "--query-db", "rds") == awsb.EXIT_FAILED
    assert harness.fleet()["phase"] == "dirty"
    assert harness.lock().exists()
    assert not runner.ran("docker start anvil")


def test_a_single_shot_creates_the_instance_measures_and_destroys(harness):
    runner = RdsRunner([DONE_STATE], describe=DESCRIBE)
    args = harness.parse("run", *harness.fleet_flags(), "--query-db", "rds")
    assert awsb.cmd_run(args, FakeSystem(run=runner)) == awsb.EXIT_OK
    assert harness.fleet()["db_modes"] == ["rds"]
    assert "rds" in harness.tfvars()
    assert not runner.ran("DROP DATABASE")
    assert runner.ran("tofu", "destroy")
    assert (harness.run_dir("01-run") / "cloudwatch/rds.json").exists()


def test_every_log_written_since_the_load_started_is_downloaded(tmp_path):
    logs = [
        {"LogFileName": "error/postgresql.log.10", "LastWritten": 50_000},
        {"LogFileName": "error/postgresql.log.11", "LastWritten": 100_000},
        {"LogFileName": "error/postgresql.log.12", "LastWritten": 9_000_000},
    ]
    runner = RdsRunner([DONE_STATE], logs=logs)
    awsb.collect_rds_logs(runner, RDS_OUTPUT, 100.0, 160.0, tmp_path)
    logs_dir = tmp_path / awsb.RDS_LOGS_DIR
    saved = sorted(p.name for p in logs_dir.iterdir())
    assert saved == ["postgresql.log.11", "postgresql.log.12"]
    text = (logs_dir / "postgresql.log.11").read_text()
    assert text == "log of error/postgresql.log.11\n"


def test_a_failed_rds_call_does_not_stop_the_other_collections(harness, case):
    runner = harness.up_rds(case)
    failing(runner, "describe-db-log-files", "logs denied")
    assert harness.run(runner, "--query-db", "rds") == awsb.EXIT_OK
    run_dir = harness.run_dir("01-rds")
    assert (run_dir / awsb.RDS_CLOUDWATCH_FILE).exists()
    assert (run_dir / awsb.EC2_NODE0_FILE).exists()
    assert not (run_dir / awsb.RDS_LOGS_DIR).exists()


@pytest.mark.parametrize(
    ("rds_min", "reason"),
    [
        ({"CPUUtilization": 12.0}, None),
        # TEST:cloudwatch-datapoint-missing-noisy
        ({"ReadIOPS": None}, "rds ReadIOPS has no datapoint"),
        ({"EBSByteBalance%": 80.0}, "rds EBSByteBalance% fell to 80%"),
        ({}, "rds metrics not collected"),
    ],
)
def test_rds_metric_gaps_are_noisy_not_invalid(rds_min, reason):
    evidence = {
        "coverage": {"ctl": 1.0, "node0": 1.0, "node1": 1.0},
        "journal_bytes": {},
        "clock_offset_ms": {},
        "digest_mismatch": {},
        "ebs_balance_min": {"EBSByteBalance%": 100.0, "EBSIOBalance%": 100.0},
        "rds_min": rds_min,
    }
    result = {**valid_result(), "hosts": {}}
    manifest = {**aws_manifest(), "query_db": "rds"}
    verdict = awsb.check_validity_aws(result, manifest, evidence)
    assert verdict["valid"]
    assert verdict["noisy"] == (reason is not None)
    if reason is not None:
        assert any(reason in r for r in verdict["reasons"])


def test_evidence_reads_the_rds_file_of_an_rds_run_only(tmp_path):
    manifest = {**aws_manifest(), "query_db": "rds"}
    _, evidence = awsb.load_evidence(tmp_path, manifest, 100.0, 160.0)
    assert evidence["rds_min"] == {}
    path = tmp_path / awsb.RDS_CLOUDWATCH_FILE
    path.parent.mkdir()
    metrics = {
        "ReadIOPS": {"timestamps": [60.0, 120.0], "values": [5.0, 3.0]},
        "WriteIOPS": {"timestamps": [], "values": []},
    }
    path.write_text(json.dumps({"period_s": 60, "metrics": metrics}))
    _, evidence = awsb.load_evidence(tmp_path, manifest, 100.0, 160.0)
    assert evidence["rds_min"] == {"ReadIOPS": 3.0, "WriteIOPS": None}
    _, evidence = awsb.load_evidence(tmp_path, aws_manifest(), 100.0, 160.0)
    assert "rds_min" not in evidence


STORE = {"gb": 400, "iops": 12000, "mbps": 500}


@pytest.mark.parametrize(
    ("query_db", "store"),
    [
        ("colocated", {"type": "root", "iops": 12000}),
        ("volume", {"type": "ebs", "volume_id": VOLUME_ID, **STORE}),
        ("rds", {"type": "rds-gp3", "identifier": "espresso-bench-run1", **STORE}),
    ],
)
def test_the_report_records_the_query_store(tmp_path, query_db, store):
    manifest = write_collected_run(tmp_path) | {
        "query_db": query_db,
        "pg_volume": STORE,
        "pg_volume_id": VOLUME_ID,
        "rds_spec": rds_spec(),
    }
    netbench.write_json(tmp_path / "manifest.json", manifest)
    meta = awsb.write_report(tmp_path)["deployment"]["query_db"]
    assert meta["mode"] == query_db
    assert meta["store"].items() >= store.items()


def test_the_result_and_summary_carry_the_block(tmp_path):
    rds = {"query_db": "rds", "rds_spec": rds_spec()}
    manifest = write_collected_run(tmp_path) | rds
    netbench.write_json(tmp_path / "manifest.json", manifest)
    series = {"timestamps": [60.0, 120.0, 180.0], "values": [1.0, 1.0, 1.0]}
    metrics = {metric: series for metric, _ in awsb.RDS_METRICS.values()}
    for balance in ("EBSIOBalance%", "EBSByteBalance%"):
        metrics[balance] = {**series, "values": [100.0] * 3}
    (tmp_path / awsb.RDS_CLOUDWATCH_FILE).write_text(
        json.dumps({"period_s": 60, "metrics": metrics})
    )
    result = awsb.write_report(tmp_path)
    query_db = result["deployment"]["query_db"]
    assert query_db["mode"] == "rds"
    assert query_db["instance_class"] == "db.m8g.4xlarge"
    assert not result["validity"]["noisy"], result["validity"]["reasons"]
    summary = (tmp_path / "summary.md").read_text()
    assert "- Query DB: rds, postgres 18.2 on db.m8g.4xlarge, rds-gp3" in summary


def rds_event(message: str, date: str) -> dict:
    return {"Message": message, "Date": date}


def down(harness: RdsHarness, runner: FakeRunner) -> dict:
    args = harness.parse("down", str(harness.fleet_dir), "--yes")
    system = FakeSystem(run=runner, clock=FakeClock())
    assert awsb.cmd_down(args, system) == awsb.EXIT_OK
    return json.loads((harness.fleet_dir / "cost.json").read_text())


# EDGE:reaper-fires-during-down
@pytest.mark.parametrize(
    "events",
    [
        [
            rds_event("DB instance shutdown", "2026-09-30T11:40:00+00:00"),
            rds_event("DB instance deleted", "2026-09-30T13:00:00+00:00"),
        ],
        [rds_event("DB instance deleted", "2026-09-30T12:55:00+00:00")],
    ],
    ids=["down-deletes", "reaper-deleted"],
)
def test_the_actual_cost_bills_the_rds_from_create_to_the_logged_delete(
    harness, case, events
):
    runner = harness.up_rds(case, events=events)
    cost = down(harness, runner)
    deleted = datetime.fromisoformat(events[-1]["Date"])
    rds_s = (deleted - datetime.fromisoformat(RDS_CREATED)).total_seconds()
    manifest = harness.fleet()
    actual = awsb.manifest_cost(manifest, cost["duration_s"], rds_s)
    assert cost["actual"] == pytest.approx(actual)
    assert cost["actual"] > awsb.manifest_cost(manifest, cost["duration_s"], 0.0)


FALLBACK = datetime(2026, 9, 30, 14, 0, tzinfo=UTC)


@pytest.mark.parametrize(
    ("events", "expected"),
    [
        ([], FALLBACK),
        (
            [
                rds_event("DB instance shutdown", "2026-09-30T13:10:00+00:00"),
                rds_event("DB instance deleted", "2026-09-30T13:00:00+00:00"),
            ],
            datetime(2026, 9, 30, 13, 0, tzinfo=UTC),
        ),
    ],
    ids=["no-event", "deletion-only"],
)
def test_only_a_deletion_event_ends_the_bill_before_the_driver_clock(events, expected):
    runner = RdsRunner([DONE_STATE], events=events)
    assert awsb.rds_delete_time(runner, "espresso-bench-fleet1", FALLBACK) == expected


def test_a_fleet_that_failed_before_the_instance_was_seen_counts_from_launch(harness):
    runner = RdsRunner(
        [DONE_STATE],
        describe=DESCRIBE,
        instances=[db_instance(applied="pending-reboot")],
        events=[rds_event("DB instance deleted", "2026-09-30T15:00:00+00:00")],
    )
    assert harness.up(runner) == awsb.EXIT_FAILED
    cost = json.loads((harness.fleet_dir / "cost.json").read_text())
    assert cost["actual"] > 0


def tf_resource(kind: str, name: str) -> str:
    """The text of one resource block of main.tf, up to its closing brace at column 0."""
    source = (awsb.TERRAFORM_SRC / "main.tf").read_text()
    start = source.index(f'resource "{kind}" "{name}" {{')
    return source[start : source.index("\n}\n", start)]


def test_the_delete_schedule_does_not_wait_for_the_instance():
    assert "aws_db_instance" not in tf_resource("aws_iam_role_policy", "scheduler")
    assert "aws_db_instance" not in tf_resource("aws_scheduler_schedule", "rds_delete")
    assert "depends_on = [aws_scheduler_schedule.rds_delete]" in tf_resource(
        "aws_db_instance", "this"
    )


def sweep_fleet1(**kwargs) -> FakeRunner:
    mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
    runner = rds_tag_runner(mappings, [], roles=[ROLE_ARN], **kwargs)
    awsb.sweep(FakeSystem(run=runner), "fleet1")
    return runner


# TEST:sweep-rds-volume-ok
def test_the_sweep_deletes_in_dependency_order_and_waits_for_the_instance():
    assert aws_verbs(sweep_fleet1()) == [
        ("resourcegroupstaggingapi", "get-resources"),
        ("scheduler", "delete-schedule-group"),
        ("ec2", "terminate-instances"),
        ("ec2", "wait"),
        ("rds", "describe-db-instances"),
        ("rds", "delete-db-instance"),
        ("rds", "wait"),
        ("rds", "delete-db-subnet-group"),
        ("rds", "delete-db-parameter-group"),
        ("ec2", "delete-volume"),
        ("ec2", "delete-security-group"),
        ("ec2", "delete-key-pair"),
        ("iam", "list-roles"),
        ("iam", "list-role-policies"),
        ("iam", "delete-role-policy"),
        ("iam", "delete-role"),
    ]


@pytest.mark.parametrize(
    ("rds_status", "waits"),
    [("deleting", True), ("", False)],
    ids=["deleting", "schedule-deleted"],
)
def test_an_instance_already_going_is_not_deleted_again(rds_status, waits):
    verbs = aws_verbs(sweep_fleet1(rds_status=rds_status))
    assert ("rds", "delete-db-instance") not in verbs
    assert (("rds", "wait") in verbs) == waits
    assert ("rds", "delete-db-subnet-group") in verbs


# TEST:sweep-iam-absent-ok
def test_a_role_that_is_already_gone_is_tolerated():
    assert ("iam", "delete-role") in aws_verbs(sweep_fleet1(role_missing=True))


def test_a_group_the_destroy_already_deleted_is_tolerated():
    mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
    runner = rds_tag_runner(mappings, [], roles=[])
    failing(runner, "delete-db-subnet-group", "DBSubnetGroupNotFoundFault")
    failing(runner, "delete-db-parameter-group", "DBParameterGroupNotFound")
    failing(runner, "delete-schedule-group", "ResourceNotFoundException")
    awsb.sweep(FakeSystem(run=runner), "fleet1")


def test_another_fleets_role_is_left_alone():
    mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
    other = "arn:aws:iam::1:role/espresso-bench/espresso-bench-other"
    runner = rds_tag_runner(mappings, [], roles=[other])
    awsb.sweep(FakeSystem(run=runner), "fleet1")
    assert ("iam", "delete-role") not in aws_verbs(runner)


def test_no_iam_permission_lists_no_roles():
    runner = rds_tag_runner([], [])
    runner.responses[LIST_ROLES] = completed(
        returncode=254, stderr="AccessDenied: iam:ListRoles"
    )
    assert awsb.list_scheduler_roles(runner) == []


def test_the_latest_expiry_of_a_partly_retagged_fleet_counts():
    mappings = [
        mapping(resource_arn("ec2", "security-group/sg-1"), "f", "bob", EXPIRES_PAST),
        mapping(resource_arn("rds", "db:espresso-bench-f"), "f", "bob", EXPIRES_LATER),
        mapping(resource_arn("ec2", "key-pair/key-1"), "f", "bob", EXPIRES_PAST),
    ]
    (tagged,) = awsb.group_runs(mappings, [], [])
    assert tagged["expires"] == EXPIRES_LATER


def test_expiries_compare_as_times_not_strings():
    earlier = "2026-09-29T09:00:00+00:00"
    assert awsb.later(earlier, "2026-09-29T10:00:00+01:00") == earlier
    assert awsb.later(None, None) is None
    assert awsb.later(None, EXPIRES_PAST) == EXPIRES_PAST


@pytest.fixture
def out_root(isolated: Path, case: unittest.TestCase) -> Path:
    return isolated_env(case)


def destroy_orphans(runner: FakeRunner) -> tuple[int, str]:
    argv = ["destroy", "--orphans", "--yes"]
    return run_cmd(awsb.cmd_destroy, argv, FakeSystem(run=runner))


def write_fleet(out: Path, name: str, phase: str) -> None:
    (out / name).mkdir(parents=True)
    netbench.write_json(out / name / "fleet.json", {"phase": phase})


def fleet1_runner(owner: str, expires: str) -> FakeRunner:
    mappings = rds_fleet_mappings("fleet1", owner, expires)
    return rds_tag_runner(mappings, [instance_row("i-1")], roles=[ROLE_ARN])


# TEST:orphans-rds-expiry-ok
def test_an_rds_fleet_past_expiry_is_listed_and_swept(out_root):
    runner = fleet1_runner("bob", EXPIRES_PAST)
    code, text = destroy_orphans(runner)
    assert code == awsb.EXIT_OK
    assert "| fleet1 | bob |" in text
    assert (
        "1 db, 1 instance, 1 key-pair, 1 pg, 1 role, 1 schedule-group, "
        "1 security-group, 1 subgrp, 1 volume"
    ) in text
    assert text.splitlines()[2].endswith("| past expiry |")
    verbs = aws_verbs(runner)
    assert ("rds", "delete-db-instance") in verbs
    assert ("iam", "delete-role") in verbs


def lone_role_runner() -> FakeRunner:
    return rds_tag_runner([], [], roles=[ROLE_ARN])


def own_fleet1_runner() -> FakeRunner:
    return fleet1_runner(FakeSystem().user(), EXPIRES_LATER)


# TEST:orphans-live-fleet-kept-ok
@pytest.mark.parametrize(
    ("make_runner", "phase"),
    [(own_fleet1_runner, "idle"), (lone_role_runner, "applying")],
    ids=["live-fleet", "role-of-fleet-provisioning"],
)
def test_a_fleet_with_local_state_is_kept(out_root, make_runner, phase):
    runner = make_runner()
    write_fleet(out_root, "fleet1", phase)
    code, text = destroy_orphans(runner)
    assert code == awsb.EXIT_OK
    assert "no orphaned" in text
    assert ("ec2", "terminate-instances") not in aws_verbs(runner)
    assert ("iam", "delete-role") not in aws_verbs(runner)


@pytest.mark.parametrize(
    ("make_runner", "reason"),
    [
        (own_fleet1_runner, "| no local state |"),
        (lone_role_runner, "| role without resources |"),
    ],
    ids=["lost-state", "role-without-resources"],
)
def test_an_own_fleet_without_local_state_is_swept(out_root, make_runner, reason):
    runner = make_runner()
    code, text = destroy_orphans(runner)
    assert code == awsb.EXIT_OK
    assert text.splitlines()[2].endswith(reason)
    assert ("iam", "delete-role") in aws_verbs(runner)


def test_a_failed_rds_delete_exits_4(out_root):
    mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_PAST)
    runner = rds_tag_runner(mappings, [], roles=[ROLE_ARN])
    failing(runner, "delete-db-instance", "InvalidDBInstanceState")
    code, _ = destroy_orphans(runner)
    assert code == awsb.EXIT_LEFTOVER


def test_an_rds_fleet_shows_the_instance_and_parameters(harness, case):
    runner = harness.up_rds(case)
    runner.describe = STATUS_DESCRIBE
    argv = ["status", str(harness.fleet_dir)]
    _, text = run_cmd(awsb.cmd_status, argv, FakeSystem(run=runner))
    assert "- rds espresso-bench-fleet1: available, parameters in-sync" in text
