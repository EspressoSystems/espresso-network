import json
import shutil
import subprocess
from pathlib import Path

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    RunHarness,
    awsb,
    completed,
    index_manifest,
    valid_result,
)

FLEET = "lulu-20261002-131457"
REJECTED = completed(returncode=1, stderr="! [rejected] HEAD -> main (fetch first)")
SECRET = "hunter2"
REMOTE = "https://u:s3cr3t@example.com/r.git"
MANIFEST = {
    **index_manifest(),
    "argv": ["run", "--tag", "t", "--yes", "--fleet", "d", "--results-remote", REMOTE],
    "fleet_argv": ["up", "--tag", "t", f"--results-remote={REMOTE}"],
    "ssh_public_key": "ssh-ed25519 AAAA",
    "config": {
        "tag": "t",
        "nodes": 3,
        "node_type": "c8g.4xlarge",
        "node_env": ["TOKEN=public"],
    },
    "hosts": [
        {"name": "ctl", "role": "ctl", "instance_type": "c8g.2xlarge", "root_gb": 40}
    ],
    "rds_spec": {"password": SECRET},
}
OLD_ROW = {
    "run": f"{FLEET}/01-run",
    "utc": "2026-10-02T13:15",
    "git": "813abe8dc8",
    "tag": "t@63b57619f4",
    "nodes": "3",
    "db": "volume",
    "rate": "170",
    "decided": "170",
    "kept_up": "yes",
    "lag_p99": "319",
    "status": "noisy",
    "exit": "0",
    "cost": "0.53",
    "user": "lulu",
}
CAPACITY = {
    "overall": {"mb_s": 140.0, "bounded": True},
    "consensus": {"mb_s": 140.0, "bounded": True},
    "query_node": {"mb_s": 120.0, "bounded": True},
}


ROW = {
    **{k: v for k, v in OLD_ROW.items() if k != "kept_up"},
    "latency": "off",
    "node_type": "c8g.4xlarge",
    "bound": "consensus",
}


def make_run_dir(root: Path, with_row: bool = True, run: str = "01-run") -> Path:
    run_dir = root / "bench-state" / "aws" / FLEET / "runs" / run
    (run_dir / "trace").mkdir(parents=True)
    (run_dir / "hosts" / "node0").mkdir(parents=True)
    (run_dir / "ssh").mkdir()
    files = {
        "summary.md": "s",
        "result.json": json.dumps({"capacity": CAPACITY}),
        "manifest.json": json.dumps(MANIFEST),
        "cost.json": "{}",
        "trace/leader_path.md": "m",
        "trace/stats.json": "{}",
        "trace/finality.png": "p",
        "trace/leader_trace_node0.csv": "big",
        "pg.json": "secret",
        "ssh/id_ed25519": "secret",
        "hosts/node0/node.env": "secret",
        "metrics.jsonl": "big",
    }
    if with_row:
        files["index-row.json"] = json.dumps({**ROW, "run": f"{FLEET}/{run}"})
    for name, text in files.items():
        (run_dir / name).write_text(text)
    return run_dir


def git_runner(*push_replies) -> FakeRunner:
    runner = FakeRunner({("git",): completed()})
    runner.respond("status --porcelain", lambda _: completed(stdout="A  runs/x\n"))
    replies = list(push_replies)
    runner.respond("push", lambda _: replies.pop(0) if replies else completed())
    return runner


def git_verbs(runner: FakeRunner) -> list[str]:
    return [call[call.index("-C") + 2] for call in runner.calls]


@pytest.fixture
def git_identity(monkeypatch: pytest.MonkeyPatch) -> None:
    """Real git without the user's config (signing, hooks)."""
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", "/dev/null")
    for key in ("AUTHOR", "COMMITTER"):
        monkeypatch.setenv(f"GIT_{key}_NAME", "t")
        monkeypatch.setenv(f"GIT_{key}_EMAIL", "t@example.com")


def git_out(*args: str | Path, check: bool = True) -> str:
    done = subprocess.run(["git", *args], check=check, capture_output=True, text=True)
    return done.stdout


def test_allowlist_excludes_secrets_and_big_files(tmp_path: Path):
    files = {str(p) for p in awsb.publish_files(make_run_dir(tmp_path))}
    assert files == {
        "summary.md",
        "result.json",
        "cost.json",
        "trace/leader_path.md",
        "trace/stats.json",
        "trace/finality.png",
    }


def test_failed_run_has_no_result_or_trace(tmp_path: Path):
    run_dir = make_run_dir(tmp_path)
    for name in (
        "result.json",
        "cost.json",
        "trace/leader_path.md",
        "trace/stats.json",
        "trace/finality.png",
    ):
        (run_dir / name).unlink()
    assert awsb.publish_files(run_dir) == [Path("summary.md")]


@pytest.mark.parametrize(
    "link", ["summary.md", "trace", "manifest.json", "result.json"]
)
def test_symlinks_are_refused(tmp_path: Path, link: str):
    run_dir = make_run_dir(tmp_path)
    target = tmp_path / "elsewhere"
    target.write_text("secret")
    path = run_dir / link
    if path.is_dir():
        shutil.rmtree(path)
    else:
        path.unlink()
    path.symlink_to(target)
    with pytest.raises(awsb.Refused, match="symlink"):
        awsb.publish_files(run_dir)


def test_published_manifest_is_an_allowlist(tmp_path: Path):
    reduced = awsb.published_manifest(make_run_dir(tmp_path))
    assert reduced == {
        "fleet": "run1",
        "created_at": MANIFEST["created_at"],
        "git_rev": MANIFEST["git_rev"],
        "query_db": "colocated",
        "images": MANIFEST["images"],
        "argv": ["run", "--tag", "t", "--fleet"],
        "fleet_argv": ["up", "--tag", "t"],
        "config": {
            "tag": "t",
            "nodes": 3,
            "node_type": "c8g.4xlarge",
            "node_env": ["TOKEN=public"],
        },
        "hosts": [{"name": "ctl", "role": "ctl", "instance_type": "c8g.2xlarge"}],
    }


def test_run_dir_shape_is_checked(tmp_path: Path):
    with pytest.raises(awsb.Refused, match="not a run dir"):
        awsb.publish_run(FakeSystem(run=git_runner()), tmp_path, "remote-url")


@pytest.mark.usefixtures("valid")
def test_index_row_json_holds_the_cells_and_the_user(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner, "--no-publish") == awsb.EXIT_OK
    row = netbench.read_json(run_harness.run_dir / "index-row.json")
    (line,) = run_harness.index().splitlines()[2:]
    assert list(row) == [*awsb.INDEX_COLUMNS, "user"]
    assert row["user"] == "tester"
    assert row["run"] == "run1/01-run"
    assert (
        line == awsb.format_index_row({k: row[k] for k in awsb.INDEX_COLUMNS}).strip()
    )


def test_index_cells_of_a_failed_run():
    cells = awsb.index_cells(index_manifest(), "01-run", None, 3, {"usd": 0.5})
    assert cells["status"] == "failed"
    assert cells["exit"] == "3"
    assert cells["cost"] == "0.50"
    assert cells["nodes"] == "5"


def test_published_row_without_new_cells_derives_them(tmp_path: Path):
    run_dir = make_run_dir(tmp_path, with_row=False)
    netbench.write_json(run_dir / "result.json", {"capacity": CAPACITY})
    netbench.write_json(
        run_dir / "index-row.json", {**OLD_ROW, "latency": "decaf-2025"}
    )
    row = awsb.read_index_row(run_dir)
    assert list(row) == [*awsb.INDEX_COLUMNS, "user"]
    assert (row["latency"], row["node_type"], row["bound"]) == (
        "decaf-2025",
        "c8g.4xlarge",
        "query",
    )


def test_published_row_keeps_cells_it_has(tmp_path: Path):
    run_dir = make_run_dir(tmp_path, with_row=False)
    row = {**OLD_ROW, "latency": "off", "node_type": "x", "bound": "both"}
    netbench.write_json(run_dir / "index-row.json", row)
    filled = awsb.read_index_row(run_dir)
    assert (filled["node_type"], filled["bound"]) == ("x", "both")


def test_row_of_a_run_without_result_has_no_bound(tmp_path: Path):
    run_dir = make_run_dir(tmp_path, with_row=False)
    (run_dir / "result.json").unlink()
    netbench.write_json(run_dir / "index-row.json", OLD_ROW)
    row = awsb.read_index_row(run_dir)
    assert (row["latency"], row["bound"]) == ("off", "-")


def test_legacy_row_with_latency_column_derives_new_cells(tmp_path: Path):
    run_dir = make_run_dir(tmp_path, with_row=False)
    netbench.write_json(run_dir / "result.json", {"capacity": CAPACITY})
    (tmp_path / "bench-state/aws/INDEX.md").write_text(
        awsb.INDEX_HEADER
        + f"| {FLEET}/01-run | 2026-10-02T13:15 | 813abe8dc8 | t@63b57619f4 | 3 | volume"
        " | off | 170 | 170 | yes | 319 | noisy | 0 | 0.53 |\n"
    )
    row = awsb.read_index_row(run_dir)
    assert list(row) == [*awsb.INDEX_COLUMNS, "user"]
    assert (row["latency"], row["node_type"], row["bound"]) == (
        "off",
        "c8g.4xlarge",
        "query",
    )


def test_legacy_row_comes_from_index_md(tmp_path: Path):
    run_dir = make_run_dir(tmp_path, with_row=False)
    other = awsb.index_cells(index_manifest(), "01-run", None, 3, None)
    (tmp_path / "bench-state/aws/INDEX.md").write_text(
        awsb.INDEX_HEADER
        + awsb.format_index_row(other)
        + f"| {FLEET}/01-run | 2026-10-02T13:15 | 813abe8dc8 | t@63b57619f4 | 3 | volume"
        " | 170 | 170 | yes | 319 | noisy | 0 | 0.53 |\n"
    )
    row = awsb.read_index_row(run_dir)
    assert row["user"] == "lulu"
    assert list(row) == [*awsb.INDEX_COLUMNS, "user"]
    assert (row["run"], row["decided"], row["lag_p99"], row["cost"]) == (
        f"{FLEET}/01-run",
        "170",
        "319",
        "0.53",
    )
    assert not (run_dir / "index-row.json").exists()


def test_legacy_run_without_a_row_is_refused(tmp_path: Path):
    run_dir = make_run_dir(tmp_path, with_row=False)
    (tmp_path / "bench-state/aws/INDEX.md").write_text(awsb.INDEX_HEADER)
    with pytest.raises(awsb.Refused, match=f"no row for {FLEET}/01-run"):
        awsb.read_index_row(run_dir)


def test_publish_clones_copies_commits_and_pushes(tmp_path: Path):
    run_dir = make_run_dir(tmp_path)
    netbench.write_json(run_dir / "index-row.json", ROW)
    runner = git_runner()
    awsb.publish_run(FakeSystem(run=runner), run_dir, "remote-url")
    assert git_verbs(runner) == [
        "clone",
        "sparse-checkout",
        "add",
        "status",
        "commit",
        "push",
    ]
    assert runner.ran("clone --depth 1 --filter=blob:none --sparse remote-url .")
    assert runner.ran("sparse-checkout set", f"runs/{FLEET}/01-run")
    assert runner.ran("http.lowSpeedLimit=1000", "http.lowSpeedTime=30")
    commit = next(c for c in runner.calls if "commit" in c)
    assert commit[-1] == f"run {FLEET}/01-run: N=3 volume 170 MB/s kept-up"
    assert runner.calls[-1][-2:] == ["origin", "HEAD:main"]


def test_publish_of_unchanged_content_is_a_noop(tmp_path: Path):
    run_dir = make_run_dir(tmp_path)
    netbench.write_json(run_dir / "index-row.json", ROW)
    runner = git_runner()
    runner.table.insert(0, ("status --porcelain", lambda _: completed()))
    awsb.publish_run(FakeSystem(run=runner), run_dir, "remote-url")
    assert git_verbs(runner) == ["clone", "sparse-checkout", "add", "status"]


def test_rejected_push_rebases_and_retries(tmp_path: Path):
    run_dir = make_run_dir(tmp_path)
    netbench.write_json(run_dir / "index-row.json", ROW)
    runner = git_runner(REJECTED, REJECTED)
    awsb.publish_run(FakeSystem(run=runner), run_dir, "remote-url")
    assert git_verbs(runner)[5:] == ["push", "pull", "push", "pull", "push"]
    assert runner.calls[6][-3:] == ["--rebase", "origin", "main"]


def test_push_gives_up_after_three_attempts(tmp_path: Path):
    run_dir = make_run_dir(tmp_path)
    netbench.write_json(run_dir / "index-row.json", ROW)
    runner = git_runner(REJECTED, REJECTED, REJECTED)
    with pytest.raises(awsb.Refused, match="push failed.*rejected"):
        awsb.publish_run(FakeSystem(run=runner), run_dir, "remote-url")
    assert runner.count("push") == 3


@pytest.mark.parametrize(
    "stderr",
    ["auth failed", "! [remote rejected] HEAD -> main (pre-receive hook declined)"],
)
def test_other_push_errors_are_not_retried(tmp_path: Path, stderr: str):
    run_dir = make_run_dir(tmp_path)
    netbench.write_json(run_dir / "index-row.json", ROW)
    runner = git_runner(completed(returncode=128, stderr=stderr))
    with pytest.raises(awsb.Refused, match="push failed"):
        awsb.publish_run(FakeSystem(run=runner), run_dir, "remote-url")
    assert runner.count("push") == 1


def test_publish_command_keeps_going_and_fails_if_any_dir_failed(tmp_path: Path):
    good = make_run_dir(tmp_path)
    netbench.write_json(good / "index-row.json", ROW)
    bad = tmp_path / "bench-state/aws/other-20261002-000000/runs/01-run"
    bad.mkdir(parents=True)
    (tmp_path / "bench-state/aws/INDEX.md").write_text(awsb.INDEX_HEADER)
    runner = git_runner()
    broken = make_run_dir(tmp_path, run="02-run")
    (broken / "summary.md").unlink()
    args = awsb.parse_args(["publish", str(bad), str(broken), str(good)])
    assert awsb.cmd_publish(args, FakeSystem(run=runner)) == awsb.EXIT_FAILED
    assert runner.count("push") == 1


def init_bare(root: Path) -> Path:
    bare = root / "bare.git"
    git_out("init", "--bare", "-b", "main", bare)
    return bare


@pytest.mark.usefixtures("git_identity")
def test_publish_to_an_empty_remote_and_again(tmp_path: Path):
    bare = init_bare(tmp_path)
    run_dir = make_run_dir(tmp_path)
    system = FakeSystem(run=awsb.host_system().run)
    for _ in range(2):
        awsb.publish_run(system, run_dir, f"file://{bare}")
    tree = git_out("-C", bare, "ls-tree", "-r", "--name-only", "main").split()
    assert f"runs/{FLEET}/01-run/summary.md" in tree
    assert not any("pg.json" in name or "ssh" in name for name in tree)
    assert git_out("-C", bare, "rev-list", "--count", "main").strip() == "1"
    published = git_out("-C", bare, "grep", "-e", SECRET, "main", check=False)
    assert published == ""
    assert git_out("-C", bare, "grep", "-e", "s3cr3t", "main", check=False) == ""


@pytest.mark.usefixtures("git_identity")
def test_rebase_after_another_publisher_pushed_first(tmp_path: Path):
    bare = init_bare(tmp_path)
    url = f"file://{bare}"
    system = FakeSystem(run=awsb.host_system().run)
    awsb.publish_run(system, make_run_dir(tmp_path), url)
    pushed = []

    def push_from_another_clone(argv, env=None):
        done = system.run(argv, env)
        if "clone" in argv and not pushed:
            pushed.append(True)
            other = tmp_path / "other"
            git_out("clone", url, other)
            (other / "other.txt").write_text("x")
            git_out("-C", other, "add", "other.txt")
            git_out("-C", other, "commit", "-m", "unrelated")
            git_out("-C", other, "push", "origin", "HEAD:main")
        return done

    stale = FakeSystem(run=push_from_another_clone)
    awsb.publish_run(stale, make_run_dir(tmp_path, run="02-run"), url)
    tree = git_out("-C", bare, "ls-tree", "-r", "--name-only", "main").split()
    assert "other.txt" in tree
    assert f"runs/{FLEET}/01-run/summary.md" in tree
    assert f"runs/{FLEET}/02-run/summary.md" in tree
    assert git_out("-C", bare, "rev-list", "--count", "main").strip() == "3"


def summary_report(run_dir: Path, baseline=None) -> dict:
    (run_dir / "summary.md").write_text("summary")
    return valid_result()


@pytest.fixture
def summary(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(awsb, "write_report", summary_report)


@pytest.mark.usefixtures("summary")
def test_run_publishes_by_default(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_OK
    assert runner.ran("git", "clone", awsb.RESULTS_REMOTE)
    assert runner.ran("git", "push", "origin", "HEAD:main")


@pytest.mark.usefixtures("summary")
def test_results_remote_flag_overrides_the_default(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    run_harness.run(runner, "--results-remote", "file:///elsewhere")
    assert runner.ran("git", "clone", "file:///elsewhere")


@pytest.mark.usefixtures("summary")
def test_no_publish_skips_git(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner, "--no-publish") == awsb.EXIT_OK
    assert not runner.ran("git", "clone")


@pytest.mark.usefixtures("summary")
def test_publish_failure_keeps_the_exit_code_and_logs_the_retry(
    run_harness: RunHarness,
):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    runner.respond("clone", lambda _: completed(returncode=128, stderr="no access"))
    assert run_harness.run(runner) == awsb.EXIT_OK
    log = run_harness.log()
    assert "WARNING publish failed: Refused: git clone failed: no access" in log
    assert f"just bench aws publish {run_harness.run_dir}" in log


def test_fleet_run_publishes_and_honors_no_publish(
    isolated: Path, monkeypatch: pytest.MonkeyPatch
):
    harness = FleetHarness(monkeypatch, isolated)
    monkeypatch.setattr(awsb, "write_report", summary_report)
    runner = harness.up_fleet()
    assert harness.run(runner, "--no-publish") == awsb.EXIT_OK
    assert not runner.ran("git", "clone")
    assert harness.run(runner) == awsb.EXIT_OK
    assert runner.ran("git", "push", "origin", "HEAD:main")
