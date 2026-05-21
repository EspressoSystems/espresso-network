#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["httpx", "rich"]
# ///
"""Binary upgrade test driver.

Boots docker-compose using docker-compose.yaml + .env from the BASE_TAG git
revision, then rolls each espresso-node-N from BASE_TAG to UPGRADE_TAG one
by one, bulk-upgrades the rest, and runs scripts/smoke-test-demo before and
after.
"""

from __future__ import annotations

import argparse
import dataclasses
import logging
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal

import httpx
from rich.logging import RichHandler

log = logging.getLogger("binary-upgrade-test")

NODE_INDICES = (0, 1, 2, 3, 4)

WIPE_FS_NODE = 4
WIPE_PG_NODE = 1
NEW_NODE_INDEX = 5
NEW_NODE_API_PORT = 24005


# Services NOT touched by the binary upgrade test:
#   - one-shots that already ran in phase 1 (deploy-*, fund-builder,
#     stake-for-demo, cdn-whitelist, wait-for-v4)
#   - infra that doesn't use an espresso-network image (postgres, keydb,
#     L1 anvil, block-explorer)
NOUPGRADE_SERVICES = (
    "block-explorer",
    "cdn-whitelist",
    "demo-l1-network",
    "deploy-espresso-contracts",
    "deploy-lcv3-upgrade",
    "deploy-pos-contracts-upgrades",
    "deploy-prover-contracts",
    "espresso-node-db-0",
    "espresso-node-db-1",
    "fund-builder",
    "keydb",
    "stake-for-demo",
    "wait-for-lc-epoch-2",
    "wait-for-v4",
)

ESPRESSO_IMAGE_PREFIX = "ghcr.io/espressosystems/espresso-network/"

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)

PERSIST_OVERLAY = REPO_ROOT / "binary-upgrade-tests" / "compose.persist-storage.yaml"
NODE_5_FS_OVERLAY = REPO_ROOT / "binary-upgrade-tests" / "compose.node-5-fs.yaml"
NODE_5_PG_OVERLAY = REPO_ROOT / "binary-upgrade-tests" / "compose.node-5-pg.yaml"
LC_GATING_OVERLAY = REPO_ROOT / "binary-upgrade-tests" / "compose.lc-gating.yaml"


YYYYMMDD_TAG_PATTERN = "20[0-9][0-9][0-1][0-9][0-3][0-9]"

# ---------------------------------------------------------------------------
# Action types
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class Roll:
    idx: int


@dataclass(frozen=True)
class Wipe:
    idx: int
    backend: Literal["fs", "pg"]


@dataclass(frozen=True)
class JoinNode:
    idx: int
    overlay: Path
    timeout: float = 300.0


@dataclass(frozen=True)
class UpgradeSupportServices:
    pass


@dataclass(frozen=True)
class AssertImages:
    pass


@dataclass(frozen=True)
class SmokeTest:
    tag_source: Literal["base", "upgrade"] = "upgrade"


Action = Roll | Wipe | JoinNode | UpgradeSupportServices | AssertImages | SmokeTest

# ---------------------------------------------------------------------------
# Scenarios
# ---------------------------------------------------------------------------

SCENARIOS: dict[str, list[Action]] = {
    "vanilla": [
        SmokeTest(tag_source="base"),
        Roll(0),
        Roll(1),
        Roll(2),
        Roll(3),
        Roll(4),
        UpgradeSupportServices(),
        AssertImages(),
        SmokeTest(),
    ],
    "new-from-old-fs": [
        SmokeTest(tag_source="base"),
        Roll(WIPE_FS_NODE),
        Wipe(WIPE_FS_NODE, backend="fs"),
        Roll(0),
        Roll(1),
        Roll(2),
        Roll(3),
        UpgradeSupportServices(),
        AssertImages(),
        SmokeTest(),
    ],
    "new-from-old-pg": [
        SmokeTest(tag_source="base"),
        Roll(WIPE_PG_NODE),
        Wipe(WIPE_PG_NODE, backend="pg"),
        Roll(0),
        Roll(2),
        Roll(3),
        Roll(4),
        UpgradeSupportServices(),
        AssertImages(),
        SmokeTest(),
    ],
    "old-from-new-fs": [
        SmokeTest(tag_source="base"),
        Roll(0),
        Roll(1),
        Roll(2),
        Roll(3),
        Roll(4),
        UpgradeSupportServices(),
        AssertImages(),
        JoinNode(NEW_NODE_INDEX, overlay=NODE_5_FS_OVERLAY),
    ],
    "old-from-new-pg": [
        SmokeTest(tag_source="base"),
        Roll(0),
        Roll(1),
        Roll(2),
        Roll(3),
        Roll(4),
        UpgradeSupportServices(),
        AssertImages(),
        JoinNode(NEW_NODE_INDEX, overlay=NODE_5_PG_OVERLAY),
    ],
}


def yyyymmdd_tags() -> list[str]:
    out = subprocess.check_output(
        ["git", "tag", "-l", YYYYMMDD_TAG_PATTERN], cwd=REPO_ROOT, text=True
    )
    return sorted(out.strip().splitlines())


def default_base_tag() -> str:
    """Pick the YYYYMMDD tag to upgrade from.

    On a tagged release build (HEAD points at a YYYYMMDD tag), use the
    previous tag so we test the new release against the prior one. Otherwise
    use the latest YYYYMMDD tag.
    """
    tags = yyyymmdd_tags()
    if not tags:
        raise RuntimeError(
            f"No tags matching {YYYYMMDD_TAG_PATTERN}; run with --tags fetched."
        )
    head_tag = subprocess.run(
        ["git", "describe", "--tags", "--exact-match"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    ).stdout.strip()
    if head_tag in tags:
        idx = tags.index(head_tag)
        if idx == 0:
            raise RuntimeError(
                f"HEAD is at {head_tag}, the oldest YYYYMMDD tag; no previous to upgrade from."
            )
        return tags[idx - 1]
    return tags[-1]


@dataclass(frozen=True)
class Config:
    base_tag: str
    upgrade_tag: str
    keep_running: bool
    upgrade_pull: bool

    @classmethod
    def from_env(cls) -> Config:
        return cls(
            base_tag=os.environ.get("BASE_TAG") or default_base_tag(),
            upgrade_tag=os.environ.get("UPGRADE_TAG", "main"),
            keep_running=os.environ.get("KEEP_RUNNING") == "1",
            upgrade_pull=os.environ.get("UPGRADE_PULL") == "1",
        )


@dataclass(frozen=True)
class Compose:
    base_dir: Path  # holds the extracted docker-compose.yaml + .env
    extra_overlays: tuple[Path, ...] = field(default_factory=tuple)

    @property
    def base_args(self) -> list[str]:
        args = [
            "docker", "compose",
            "--project-directory", str(REPO_ROOT),
            "--env-file", str(self.base_dir / ".env"),
            "-f", str(self.base_dir / "docker-compose.yaml"),
            "-f", str(PERSIST_OVERLAY),
        ]  # fmt: skip
        if (
            "wait-for-lc-epoch"
            not in (self.base_dir / "docker-compose.yaml").read_text()
        ):
            args += ["-f", str(LC_GATING_OVERLAY)]
        for overlay in self.extra_overlays:
            args += ["-f", str(overlay)]
        return args

    def with_overlays(self, *paths: Path) -> Compose:
        return dataclasses.replace(
            self, extra_overlays=self.extra_overlays + tuple(paths)
        )

    def run(
        self,
        *args: str,
        docker_tag: str | None = None,
        check: bool = True,
        capture: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ.copy()
        if docker_tag is not None:
            env["DOCKER_TAG"] = docker_tag
        return subprocess.run(
            self.base_args + list(args),
            cwd=REPO_ROOT,
            env=env,
            check=check,
            capture_output=capture,
            text=True,
        )

    def services(self) -> list[str]:
        out = self.run("config", "--services", capture=True).stdout
        return [s for s in out.splitlines() if s]

    def container_id(self, service: str) -> str:
        return self.run("ps", "-q", service, capture=True).stdout.strip()

    def find_container(self, service: str) -> str | None:
        """Find a container by service name via `docker ps` (sees overlay-added services)."""
        out = subprocess.run(
            [
                "docker",
                "ps",
                "-a",
                "--filter",
                f"name={service}",
                "--format",
                "{{.Names}}",
            ],
            capture_output=True,
            text=True,
            check=False,
        ).stdout
        for name in out.splitlines():
            if name.endswith(f"-{service}-1"):
                return name
        return None

    def container_status(self, service: str) -> str | None:
        """docker State.Status: running, exited, ... or None if no container."""
        name = self.find_container(service)
        if not name:
            return None
        result = subprocess.run(
            ["docker", "inspect", "-f", "{{.State.Status}}", name],
            capture_output=True,
            text=True,
            check=False,
        )
        return result.stdout.strip() or None

    def dump_service_logs(self, service: str, tail: int = 1000) -> None:
        name = self.find_container(service)
        if not name:
            log.error(f"--- no container found for service {service} ---")
            return
        log.error(f"--- docker logs --tail {tail} {name} ---")
        subprocess.run(["docker", "logs", "--tail", str(tail), name], check=False)

    def dump_all_logs(self, dest_dir: Path) -> None:
        """Per-service logs + ps state + the background `compose up` log."""
        dest_dir.mkdir(parents=True, exist_ok=True)
        try:
            services = self.services()
        except subprocess.CalledProcessError:
            log.warning("compose config --services failed; skipping log dump")
            return
        with (dest_dir / "ps.txt").open("wb") as f:
            subprocess.run(
                self.base_args + ["ps", "-a"],
                cwd=REPO_ROOT,
                env=os.environ.copy(),
                stdout=f,
                stderr=subprocess.STDOUT,
                check=False,
            )
        for service in services:
            with (dest_dir / f"{service}.log").open("wb") as f:
                subprocess.run(
                    self.base_args + ["logs", "--no-color", service],
                    cwd=REPO_ROOT,
                    env=os.environ.copy(),
                    stdout=f,
                    stderr=subprocess.STDOUT,
                    check=False,
                )
        compose_up_log = self.base_dir / "compose-up.log"
        if compose_up_log.exists():
            shutil.copy(compose_up_log, dest_dir / "compose-up.log")

    def upgraded_services(self) -> list[str]:
        return [s for s in self.services() if s not in NOUPGRADE_SERVICES]

    def pull(self, *tags: str, retries: int = 4, backoff: float = 10.0) -> None:
        # ghcr.io occasionally returns "context deadline exceeded" on manifest
        # HEADs when many parallel CI jobs pull at once. `--policy missing`
        # makes retries cheap: already-pulled images are skipped.
        for tag in tags:
            log.info(f"Pulling images (DOCKER_TAG={tag})")
            for attempt in range(1, retries + 1):
                result = self.run(
                    "pull", "--policy", "missing", docker_tag=tag, check=False
                )
                if result.returncode == 0:
                    break
                if attempt == retries:
                    raise RuntimeError(
                        f"compose pull failed after {retries} attempts for tag={tag}"
                    )
                log.warning(
                    f"compose pull attempt {attempt}/{retries} for tag={tag} failed; retrying in {backoff:g}s"
                )
                time.sleep(backoff)

    def assert_all_espresso_images(self, expected_tag: str) -> None:
        bad: list[str] = []
        for service in self.upgraded_services():
            cid = self.container_id(service)
            if not cid:
                continue
            image = subprocess.check_output(
                ["docker", "inspect", cid, "--format={{.Config.Image}}"],
                text=True,
            ).strip()
            if not image.startswith(ESPRESSO_IMAGE_PREFIX):
                continue
            if image.endswith(f":{expected_tag}"):
                log.info(f"service {service} image: {image}")
            else:
                log.error(
                    f"service {service} image is {image}, expected tag {expected_tag}"
                )
                bad.append(service)
        if bad:
            raise RuntimeError(f"Wrong tag on services: {', '.join(bad)}")

    def bulk_upgrade_remaining(self, upgrade_tag: str) -> None:
        node_services = tuple(f"espresso-node-{i}" for i in NODE_INDICES)
        services = [s for s in self.upgraded_services() if s not in node_services]
        if not services:
            raise RuntimeError("No remaining services to upgrade")
        log.info(
            f"Bulk-upgrading {len(services)} services to {upgrade_tag}: {' '.join(services)}"
        )
        self.run("up", "-d", "--no-deps", *services, docker_tag=upgrade_tag)

    def stop_and_remove_service(self, service: str) -> None:
        log.info(f"Stopping and removing service {service}")
        self.run("rm", "-fsv", service)

    def wipe_fs_node(self, idx: int) -> None:
        self.stop_and_remove_service(f"espresso-node-{idx}")
        remove_named_volume(fs_volume_name(idx))

    def wipe_pg_node(self, idx: int) -> None:
        # `compose rm -fsv` removes anonymous volumes attached to the postgres
        # container, wiping its data. If a named volume is ever declared for the
        # db service, this stops wiping and needs an explicit `docker volume rm`.
        db_service = f"espresso-node-db-{idx}"
        self.stop_and_remove_service(f"espresso-node-{idx}")
        self.stop_and_remove_service(db_service)
        log.info(f"Restarting fresh {db_service}")
        self.run("up", "-d", "--no-deps", db_service)
        try:
            poll_until(
                lambda: _db_container_healthy(db_service),
                f"{db_service} healthy",
                timeout=60,
            )
        except TimeoutError:
            self.dump_service_logs(db_service)
            raise

    def smoke_test(self, tag: str) -> None:
        # cwd=base_dir so the script's only-if-unset .env loader picks up the
        # extracted base-tag .env. Scrub ESPRESSO_*/ESP_* from the subprocess
        # env: REPO_ROOT/.env (loaded into os.environ by load_project_env)
        # carries main's renamed vars, which would otherwise override
        # base-tag values and point the smoke test at the wrong addresses.
        env = {
            k: v
            for k, v in os.environ.items()
            if not k.startswith(("ESPRESSO_", "ESP_"))
        }
        env["DOCKER_TAG"] = tag
        subprocess.run(
            ["timeout", "600", str(REPO_ROOT / "scripts" / "smoke-test-demo")],
            cwd=self.base_dir,
            env=env,
            check=True,
        )


def _get(url: str, timeout: float = 5.0) -> tuple[int, str]:
    try:
        r = httpx.get(url, timeout=timeout)
        return r.status_code, r.text.strip()
    except httpx.RequestError:
        return 0, ""


def poll_until(
    check: Callable[[], bool],
    desc: str,
    timeout: float,
    interval: float = 2.0,
    abort: Callable[[], str | None] | None = None,
) -> None:
    """Poll `check` until True or `timeout`. `abort` returning a string fails fast."""
    deadline = time.monotonic() + timeout
    while True:
        if check():
            return
        if abort is not None and (reason := abort()) is not None:
            raise RuntimeError(f"Aborted waiting for {desc}: {reason}")
        if time.monotonic() > deadline:
            raise TimeoutError(f"Timed out after {timeout:g}s waiting for {desc}")
        time.sleep(interval)


@dataclass(frozen=True)
class Node:
    """An espresso-node-N instance reachable via its host API port."""

    index: int
    api_url: str

    def __str__(self) -> str:
        return f"espresso-node-{self.index}"

    @property
    def has_query(self) -> bool:
        """Node 2 runs only the `status` module; the others run `query`."""
        return self.index != 2

    @classmethod
    def from_index(cls, index: int) -> Node:
        var = f"ESPRESSO_NODE_{index}_API_PORT"
        port = os.environ.get(var)
        if not port:
            raise RuntimeError(f"Env var {var} not set")
        return cls(index=index, api_url=f"http://localhost:{port}")

    def storage_height(self) -> int | None:
        """/node/block-height — proves consensus advanced AND indexer DB kept up."""
        code, body = _get(f"{self.api_url}/node/block-height")
        h = int(body) if code == 200 and body.isdigit() else None
        if h is None:
            log.debug(f"poll {self}/node/block-height -> {code} {body[:80]!r}")
        else:
            log.debug(f"poll {self}/node/block-height -> {h}")
        return h

    def consensus_height(self) -> int | None:
        """/status/block-height — proves consensus advanced only."""
        code, body = _get(f"{self.api_url}/status/block-height")
        h = int(body) if code == 200 and body.isdigit() else None
        if h is None:
            log.debug(f"poll {self}/status/block-height -> {code} {body[:80]!r}")
        else:
            log.debug(f"poll {self}/status/block-height -> {h}")
        return h

    def leaf_available(self, index: int) -> bool:
        code, _ = _get(f"{self.api_url}/availability/leaf/{index}")
        return code == 200

    def wait_consensus(
        self,
        target: int,
        timeout: float,
        container_status: Callable[[], str | None] | None = None,
    ) -> None:
        last_h: int | None = None

        def check() -> bool:
            nonlocal last_h
            h = self.consensus_height()
            if h is not None:
                last_h = h
            return h is not None and h >= target

        abort = _make_abort(self, container_status)
        try:
            poll_until(check, f"{self} consensus >= {target}", timeout, abort=abort)
        except TimeoutError:
            secs = int(timeout)
            if last_h is None or last_h < 5:
                raise TimeoutError(f"{self} did not join consensus after {secs}s")
            raise TimeoutError(
                f"{self} consensus stalled after {secs}s (height={last_h}, target={target})"
            )

    def wait_storage(
        self,
        target: int,
        timeout: float,
        container_status: Callable[[], str | None] | None = None,
    ) -> None:
        assert self.has_query, f"{self} does not have query API"
        last_cons: int | None = None
        last_stor: int | None = None

        def check() -> bool:
            nonlocal last_cons, last_stor
            c = self.consensus_height()
            s = self.storage_height()
            if c is not None:
                last_cons = c
            if s is not None:
                last_stor = s
            return s is not None and s >= target

        abort = _make_abort(self, container_status)
        try:
            poll_until(check, f"{self} storage >= {target}", timeout, abort=abort)
        except TimeoutError:
            secs = int(timeout)
            if last_cons is None or last_cons < 5:
                raise TimeoutError(f"{self} did not join consensus after {secs}s")
            if last_stor is not None and last_cons - last_stor > 5:
                raise TimeoutError(
                    f"{self} storage not catching up after {secs}s"
                    f" (consensus={last_cons}, storage={last_stor})"
                )
            raise TimeoutError(
                f"{self} storage stalled after {secs}s (storage={last_stor}, target={target})"
            )

        # leaf availability check
        idx = target - 1
        leaf_timeout = 30.0
        stor_at_leaf = last_stor

        def leaf_ok() -> bool:
            if self.leaf_available(idx):
                log.info(f"{self} availability/leaf/{idx} ok")
                return True
            return False

        try:
            poll_until(leaf_ok, f"{self} leaf/{idx}", leaf_timeout, abort=abort)
        except TimeoutError:
            raise TimeoutError(
                f"{self} leaf/{idx} not available after {int(leaf_timeout)}s"
                f" (storage_height={stor_at_leaf})"
            )


def _make_abort(
    node: Node, container_status: Callable[[], str | None] | None
) -> Callable[[], str | None] | None:
    if container_status is None:
        return None

    def abort() -> str | None:
        status = container_status()
        if status not in (None, "running", "created", "restarting"):
            return f"{node} container status is {status}"
        return None

    return abort


def fs_volume_name(idx: int) -> str:
    return f"espresso-node-{idx}-storage"


def remove_named_volume(name: str) -> None:
    log.info(f"Removing docker volume {name}")
    subprocess.run(["docker", "volume", "rm", "-f", name], check=True)


def _db_container_healthy(service: str) -> bool:
    cid_out = subprocess.run(
        ["docker", "ps", "-q", "-f", f"name={service}"],
        capture_output=True,
        text=True,
        check=False,
    )
    cid = cid_out.stdout.strip().splitlines()
    if not cid:
        return False
    health = subprocess.run(
        ["docker", "inspect", "--format={{.State.Health.Status}}", cid[0]],
        capture_output=True,
        text=True,
        check=False,
    )
    return health.stdout.strip() == "healthy"


def extract_base_files(base_tag: str, base_dir: Path) -> None:
    for name in ("docker-compose.yaml", ".env"):
        content = subprocess.check_output(
            ["git", "show", f"{base_tag}:{name}"],
            cwd=REPO_ROOT,
            text=True,
        )
        (base_dir / name).write_text(content)


def load_project_env() -> None:
    """Source the repo .env via bash so ${VAR} interpolation works, then copy
    .env-defined keys into os.environ without clobbering existing values
    (so callers can override individual keys from the outer shell)."""
    path = REPO_ROOT / ".env"
    keys: set[str] = set()
    for raw in path.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        keys.add(line.split("=", 1)[0].strip())

    result = subprocess.run(
        ["bash", "-c", f"set -a; . {shlex.quote(str(path))}; env -0"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
    )
    for entry in result.stdout.decode().split("\0"):
        k, sep, v = entry.partition("=")
        if sep and k in keys:
            os.environ.setdefault(k, v)


@contextmanager
def compose_session(config: Config):
    base_dir = Path(tempfile.mkdtemp(prefix="espresso-binary-upgrade-test."))
    compose = Compose(base_dir=base_dir)
    log.info(
        f"Extracting docker-compose.yaml and .env from git ref {config.base_tag} into {base_dir}"
    )
    extract_base_files(config.base_tag, base_dir)
    try:
        yield compose
    finally:
        logs_dir = REPO_ROOT / "tmp" / "compose-logs"
        log.info(f"Archiving compose logs to {logs_dir}")
        compose.dump_all_logs(logs_dir)
        if config.keep_running:
            log.info(f"KEEP_RUNNING=1, leaving compose stack up at {base_dir}")
            return
        log.info("Tearing down compose stack")
        compose.run("down", "-v", check=False, capture=True)
        shutil.rmtree(base_dir, ignore_errors=True)


# ---------------------------------------------------------------------------
# Action implementations
# ---------------------------------------------------------------------------


def _boot_network(compose: Compose, config: Config) -> None:
    compose.run("down", "-v", check=False, capture=True)
    log.info(f"Starting network on {config.base_tag}")
    # `compose up -d` blocks on `depends_on: service_completed_successfully`,
    # but `deploy-lcv3-upgrade` retries forever and `prover-one-shot` has a
    # broken healthcheck, so a synchronous call would never return. Run in
    # the background; the smoke test that follows verifies readiness end-to-end.
    compose_up_log = compose.base_dir / "compose-up.log"
    with compose_up_log.open("wb") as f:
        subprocess.Popen(
            compose.base_args + ["up", "-d"],
            cwd=REPO_ROOT,
            env=os.environ | {"DOCKER_TAG": config.base_tag},
            stdout=f,
            stderr=subprocess.STDOUT,
        )
    log.info(f"compose up -d running in background; log at {compose_up_log}")


def _roll(compose: Compose, idx: int, upgrade_tag: str) -> None:
    nodes = [Node.from_index(i) for i in NODE_INDICES]
    ref = nodes[1] if idx == 0 else nodes[0]
    initial: int | None = None
    for _ in range(5):
        initial = ref.storage_height()
        if initial is not None:
            break
        time.sleep(2)
    if initial is None:
        raise RuntimeError(f"Could not read reference height from {ref}")
    target = initial + 2

    log.info(
        f"Rolling espresso-node-{idx} to {upgrade_tag}; waiting for all nodes to reach height {target}"
    )
    compose.run(
        "up",
        "-d",
        "--no-deps",
        "--force-recreate",
        f"espresso-node-{idx}",
        docker_tag=upgrade_tag,
    )

    for node in nodes:
        try:
            node.wait_consensus(target, timeout=120)
        except TimeoutError:
            compose.dump_service_logs(str(node))
            raise


def _restart_with_config_peer(compose: Compose, idx: int, tag: str) -> None:
    peer_idx = 1 if idx == 0 else 0
    peer_port_var = f"ESPRESSO_NODE_{peer_idx}_API_PORT"
    peer_port = os.environ.get(peer_port_var)
    if not peer_port:
        raise RuntimeError(f"Env var {peer_port_var} not set")
    peer_url = f"http://espresso-node-{peer_idx}:{peer_port}"
    overlay = compose.base_dir / f"restart-node-{idx}.yaml"
    overlay.write_text(
        f"services:\n  espresso-node-{idx}:\n    environment:\n      ESPRESSO_NODE_CONFIG_PEERS: {peer_url}\n"
    )
    log.info(
        f"Restarting espresso-node-{idx} with ESPRESSO_NODE_CONFIG_PEERS={peer_url} on tag {tag}"
    )
    compose.with_overlays(overlay).run(
        "up",
        "-d",
        "--no-deps",
        "--force-recreate",
        f"espresso-node-{idx}",
        docker_tag=tag,
    )


def _wait_storage(compose: Compose, idx: int, timeout: float) -> None:
    peer_idxs = tuple(i for i in NODE_INDICES if i != idx)
    peers = [n for i in peer_idxs if (n := Node.from_index(i)).has_query]
    heights = [h for p in peers if (h := p.storage_height()) is not None]
    if not heights:
        raise RuntimeError("No peer reported a storage height")
    target = max(heights) + 2
    node = Node.from_index(idx)
    log.info(f"Waiting for {node} to catch up to storage height {target}")
    try:
        node.wait_storage(
            target,
            timeout,
            container_status=lambda: compose.container_status(str(node)),
        )
    except (TimeoutError, RuntimeError):
        compose.dump_service_logs(str(node))
        raise


def _execute(action: Action, compose: Compose, config: Config) -> None:
    match action:
        case Roll(idx=idx):
            _roll(compose, idx, config.upgrade_tag)

        case Wipe(idx=idx, backend=backend):
            if backend == "fs":
                compose.wipe_fs_node(idx)
            else:
                compose.wipe_pg_node(idx)
            _restart_with_config_peer(compose, idx, config.upgrade_tag)
            _wait_storage(compose, idx, timeout=240.0)

        case JoinNode(idx=idx, overlay=overlay, timeout=timeout):
            os.environ[f"ESPRESSO_NODE_{idx}_API_PORT"] = str(NEW_NODE_API_PORT)
            log.info(f"Starting espresso-node-{idx} on tag {config.base_tag}")
            compose.with_overlays(overlay).run(
                "up", "-d", f"espresso-node-{idx}", docker_tag=config.base_tag
            )
            _wait_storage(compose, idx, timeout=timeout)

        case UpgradeSupportServices():
            log.info(f"Bulk-upgrading remaining services to {config.upgrade_tag}")
            compose.bulk_upgrade_remaining(config.upgrade_tag)

        case AssertImages():
            log.info(
                f"Asserting all espresso-network images run tag {config.upgrade_tag}"
            )
            compose.assert_all_espresso_images(config.upgrade_tag)

        case SmokeTest(tag_source=src):
            tag = config.base_tag if src == "base" else config.upgrade_tag
            log.info(f"Smoke test (tag={tag})")
            compose.smoke_test(tag)

        case _:
            raise ValueError(f"Unknown action: {action}")


def run_scenario(actions: list[Action], compose: Compose, config: Config) -> None:
    _boot_network(compose, config)
    for action in actions:
        _execute(action, compose, config)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Binary upgrade test driver")
    parser.add_argument("--log-level", default="INFO")
    parser.add_argument(
        "--scenario",
        choices=list(SCENARIOS),
        default="vanilla",
    )
    parser.add_argument(
        "--pull-only",
        action="store_true",
        help="Pull base and (if UPGRADE_PULL=1) upgrade images, then exit.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    logging.basicConfig(
        handlers=[RichHandler(show_path=False)],
        level=args.log_level,
        format="%(message)s",
    )

    if not (REPO_ROOT / ".env").exists():
        log.error(".env not found. Copy .env.docker.example to .env first.")
        return 1

    config = Config.from_env()
    log.info(f"BASE_TAG={config.base_tag} UPGRADE_TAG={config.upgrade_tag}")
    os.environ.setdefault(
        "ESPRESSO_SEQUENCER_GENESIS_FILE", "genesis/demo-drb-header.toml"
    )
    os.environ.setdefault("ESPRESSO_NODE_GENESIS_FILE", "genesis/demo-drb-header.toml")
    load_project_env()

    with compose_session(config) as compose:
        tags = (config.base_tag,)
        if config.upgrade_pull:
            tags += (config.upgrade_tag,)
        compose.pull(*tags)

        if args.pull_only:
            log.info("--pull-only: images pulled, exiting before stack start")
            return 0

        log.info(f"Running scenario: {args.scenario}")
        run_scenario(SCENARIOS[args.scenario], compose, config)
        log.info("Binary upgrade test complete")
    return 0


if __name__ == "__main__":
    sys.exit(main())
