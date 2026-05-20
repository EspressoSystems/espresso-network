#!/usr/bin/env python3
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
import urllib.error
import urllib.request
from collections.abc import Callable
from contextlib import contextmanager
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path

log = logging.getLogger("binary-upgrade-test")

NODE_INDICES = (0, 1, 2, 3, 4)

WIPE_FS_NODE = 4
WIPE_PG_NODE = 1
NEW_NODE_INDEX = 5
NEW_NODE_API_PORT = 24005


class Scenario(StrEnum):
    VANILLA = "vanilla"
    NEW_FROM_OLD_FS = "new-from-old-fs"
    NEW_FROM_OLD_PG = "new-from-old-pg"
    OLD_FROM_NEW_FS = "old-from-new-fs"
    OLD_FROM_NEW_PG = "old-from-new-pg"


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
            ["docker", "ps", "-a", "--filter", f"name={service}", "--format", "{{.Names}}"],
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
        subprocess.run(
            ["docker", "logs", "--tail", str(tail), name], check=False
        )

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

    def wait_for_catchup(
        self, idx: int, peers: list[Node], timeout: float = 240
    ) -> None:
        heights = [p.storage_height() for p in peers if p.has_query]
        heights = [h for h in heights if h is not None]
        if not heights:
            raise RuntimeError("No peer reported a storage height")
        target = max(heights) + 2
        node = Node.from_index(idx)
        log.info(f"Waiting for {node} to catch up to height {target}")
        try:
            node.wait_until_at_height(
                target,
                height_timeout=timeout,
                leaf_timeout=timeout,
                container_status=lambda: self.container_status(str(node)),
            )
        except (TimeoutError, RuntimeError):
            self.dump_service_logs(str(node))
            raise

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

    def roll_node(self, n: int, upgrade_tag: str) -> None:
        nodes = [Node.from_index(i) for i in NODE_INDICES]
        ref = nodes[1] if n == 0 else nodes[0]
        initial = ref.storage_height()
        if initial is None:
            raise RuntimeError(f"Could not read reference height from {ref}")
        target = initial + 2

        log.info(
            f"Recreating {nodes[n]} with tag={upgrade_tag}; waiting for all nodes to reach height {target}"
        )
        # No --wait: the new image's baked-in healthcheck reads ESPRESSO_NODE_API_PORT
        # but the old compose only sets ESPRESSO_SEQUENCER_API_PORT. The polling
        # below verifies consensus liveness directly.
        self.run(
            "up",
            "-d",
            "--no-deps",
            "--force-recreate",
            str(nodes[n]),
            docker_tag=upgrade_tag,
        )

        for node in nodes:
            try:
                node.wait_until_at_height(target)
            except TimeoutError:
                self.dump_service_logs(str(node))
                raise

    def roll_all_nodes(self, upgrade_tag: str, skip: tuple[int, ...] = ()) -> None:
        for n in NODE_INDICES:
            if n in skip:
                continue
            log.info(f"Rolling espresso-node-{n} to {upgrade_tag}")
            self.roll_node(n, upgrade_tag)

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

    def restart_node_with_config_peer(self, idx: int, tag: str) -> None:
        # The CONFIG_PEERS overlay applies only to this single `up`; if the same
        # node were rolled again afterwards without this overlay, it would lose
        # the env var. Scenarios that call this never re-roll the wiped node.
        peer_idx = 1 if idx == 0 else 0
        peer_port_var = f"ESPRESSO_NODE_{peer_idx}_API_PORT"
        peer_port = os.environ.get(peer_port_var)
        if not peer_port:
            raise RuntimeError(f"Env var {peer_port_var} not set")
        peer_url = f"http://espresso-node-{peer_idx}:{peer_port}"

        overlay = self.base_dir / f"restart-node-{idx}.yaml"
        overlay.write_text(
            f"""services:
  espresso-node-{idx}:
    environment:
      ESPRESSO_NODE_CONFIG_PEERS: {peer_url}
"""
        )
        log.info(
            f"Restarting espresso-node-{idx} with ESPRESSO_NODE_CONFIG_PEERS={peer_url} on tag {tag}"
        )
        self.with_overlays(overlay).run(
            "up",
            "-d",
            "--no-deps",
            "--force-recreate",
            f"espresso-node-{idx}",
            docker_tag=tag,
        )

    def start_new_node_5(self, base_tag: str, overlay: Path) -> Node:
        keys = _generate_keys(base_tag)
        os.environ[f"ESPRESSO_NODE_{NEW_NODE_INDEX}_API_PORT"] = str(NEW_NODE_API_PORT)
        os.environ[f"ESPRESSO_NODE_{NEW_NODE_INDEX}_STAKING_PRIVATE_KEY"] = keys[
            "ESPRESSO_NODE_PRIVATE_STAKING_KEY"
        ]
        os.environ[f"ESPRESSO_NODE_{NEW_NODE_INDEX}_STATE_PRIVATE_KEY"] = keys[
            "ESPRESSO_NODE_PRIVATE_STATE_KEY"
        ]
        os.environ[f"ESPRESSO_NODE_{NEW_NODE_INDEX}_X25519_PRIVATE_KEY"] = keys[
            "ESPRESSO_NODE_PRIVATE_X25519_KEY"
        ]
        log.info(f"Starting espresso-node-{NEW_NODE_INDEX} on tag {base_tag}")
        self.with_overlays(overlay).run(
            "up",
            "-d",
            f"espresso-node-{NEW_NODE_INDEX}",
            docker_tag=base_tag,
        )
        return Node.from_index(NEW_NODE_INDEX)

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

    def boot_base_network(self, config: Config) -> None:
        self.run("down", "-v", check=False, capture=True)

        log.info(f"Starting network on {config.base_tag}")
        # `compose up -d` blocks on `depends_on: service_completed_successfully`,
        # but `deploy-lcv3-upgrade` retries forever and `prover-one-shot` has a
        # broken healthcheck, so a synchronous call would never return. Run in
        # the background and let the smoke test below verify readiness end to
        # end. The compose stack is torn down on context exit regardless.
        compose_up_log = self.base_dir / "compose-up.log"
        with compose_up_log.open("wb") as f:
            subprocess.Popen(
                self.base_args + ["up", "-d"],
                cwd=REPO_ROOT,
                env=os.environ | {"DOCKER_TAG": config.base_tag},
                stdout=f,
                stderr=subprocess.STDOUT,
            )
        log.info(f"compose up -d running in background; log at {compose_up_log}")

        log.info("Initial smoke test")
        self.smoke_test(config.base_tag)

    def run_full_vanilla_upgrade(self, config: Config) -> None:
        self.roll_all_nodes(config.upgrade_tag)

        log.info(f"Bulk-upgrading remaining services to {config.upgrade_tag}")
        self.bulk_upgrade_remaining(config.upgrade_tag)

        log.info(f"Asserting all espresso-network images run tag {config.upgrade_tag}")
        self.assert_all_espresso_images(config.upgrade_tag)

    def vanilla(self, config: Config) -> None:
        self.boot_base_network(config)
        self.run_full_vanilla_upgrade(config)

        log.info("Final smoke test")
        self.smoke_test(config.upgrade_tag)

    def new_from_old(
        self,
        config: Config,
        wipe_idx: int,
        wipe: Callable[[int], None],
    ) -> None:
        """New (UPGRADE) node catches up from old (BASE) peers.

        Rolls just `wipe_idx` to UPGRADE, wipes it, restarts; the remaining
        4 nodes are still on BASE_TAG when catchup runs. Then finishes the
        rolling upgrade for the other nodes.
        """
        self.boot_base_network(config)

        log.info(f"Rolling espresso-node-{wipe_idx} to {config.upgrade_tag}")
        self.roll_node(wipe_idx, config.upgrade_tag)

        wipe(wipe_idx)
        self.restart_node_with_config_peer(wipe_idx, config.upgrade_tag)

        peers = [Node.from_index(i) for i in NODE_INDICES if i != wipe_idx]
        self.wait_for_catchup(wipe_idx, peers)

        self.roll_all_nodes(config.upgrade_tag, skip=(wipe_idx,))

        log.info(f"Bulk-upgrading remaining services to {config.upgrade_tag}")
        self.bulk_upgrade_remaining(config.upgrade_tag)

        log.info(f"Asserting all espresso-network images run tag {config.upgrade_tag}")
        self.assert_all_espresso_images(config.upgrade_tag)

        log.info("Final smoke test")
        self.smoke_test(config.upgrade_tag)

    def old_from_new(self, config: Config, node_5_overlay: Path) -> None:
        """Old (BASE) node catches up from new (UPGRADE) peers.

        Finishes the full upgrade, then starts a fresh espresso-node-5 on
        BASE_TAG. Verifies the upgraded peers can still serve a base-version
        client (API/wire compatibility).
        """
        self.boot_base_network(config)
        self.run_full_vanilla_upgrade(config)

        self.start_new_node_5(config.base_tag, node_5_overlay)
        peers = [Node.from_index(i) for i in NODE_INDICES]
        self.wait_for_catchup(NEW_NODE_INDEX, peers, timeout=300)


def _http_status_and_body(url: str, timeout: float = 5.0) -> tuple[int, str]:
    try:
        with urllib.request.urlopen(url, timeout=timeout) as resp:
            return resp.status, resp.read().decode().strip()
    except urllib.error.HTTPError as e:
        return e.code, ""
    except (urllib.error.URLError, TimeoutError, ConnectionError, OSError):
        return 0, ""


def _height_at(api_url: str, path: str) -> int | None:
    url = f"{api_url}{path}"
    code, body = _http_status_and_body(url)
    height = int(body) if code == 200 and body.isdigit() else None
    if height is None:
        snippet = body[:120].replace("\n", " ")
        log.info(f"poll {url} -> status={code} body={snippet!r}")
    else:
        log.info(f"poll {url} -> {height}")
    return height


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
        return _height_at(self.api_url, "/node/block-height")

    def consensus_height(self) -> int | None:
        """/status/block-height — proves consensus advanced only."""
        return _height_at(self.api_url, "/status/block-height")

    def leaf_available(self, index: int) -> bool:
        code, _ = _http_status_and_body(f"{self.api_url}/availability/leaf/{index}")
        return code == 200

    def wait_until_at_height(
        self,
        target: int,
        height_timeout: float = 120,
        leaf_timeout: float = 30,
        container_status: Callable[[], str | None] | None = None,
    ) -> None:
        if self.has_query:
            name, getter = "storage_height", self.storage_height
        else:
            name, getter = "consensus_height", self.consensus_height

        def height_ok() -> bool:
            h = getter()
            if h is not None and h >= target:
                log.info(f"{self} {name} {h} >= {target}")
                return True
            return False

        abort = None
        if container_status is not None:

            def abort() -> str | None:
                status = container_status()
                if status not in (None, "running", "created", "restarting"):
                    return f"{self} container status is {status}"
                return None

        poll_until(height_ok, f"{self} {name} >= {target}", height_timeout, abort=abort)

        if self.has_query:
            idx = target - 1

            def leaf_ok() -> bool:
                if self.leaf_available(idx):
                    log.info(f"{self} availability/leaf/{idx} ok")
                    return True
                return False

            poll_until(
                leaf_ok, f"{self} availability/leaf/{idx}", leaf_timeout, abort=abort
            )


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


def _generate_keys(image_tag: str) -> dict[str, str]:
    image = f"{ESPRESSO_IMAGE_PREFIX}espresso-node:{image_tag}"
    log.info(
        f"Generating fresh keypair for espresso-node-{NEW_NODE_INDEX} using {image}"
    )
    result = subprocess.run(
        ["docker", "run", "--rm", "--entrypoint=/bin/keygen", image, "--scheme", "all"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"keygen failed (exit {result.returncode}) in {image}; "
            "older BASE_TAGs without /bin/keygen are unsupported. "
            f"stderr:\n{result.stderr}"
        )
    raw: dict[str, str] = {}
    for line in result.stdout.splitlines():
        if "=" not in line or line.startswith("#"):
            continue
        k, _, v = line.partition("=")
        raw[k.strip()] = v.strip()
    # keygen output renamed ESPRESSO_SEQUENCER_* -> ESPRESSO_NODE_* in #4111;
    # accept either so the helper works across BASE_TAG and UPGRADE_TAG.
    aliases = {
        "ESPRESSO_NODE_PRIVATE_STAKING_KEY": "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY",
        "ESPRESSO_NODE_PRIVATE_STATE_KEY": "ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY",
        "ESPRESSO_NODE_PRIVATE_X25519_KEY": "ESPRESSO_SEQUENCER_PRIVATE_X25519_KEY",
    }
    out: dict[str, str] = {}
    missing: list[str] = []
    for new_name, old_name in aliases.items():
        if new_name in raw:
            out[new_name] = raw[new_name]
        elif old_name in raw:
            out[new_name] = raw[old_name]
        else:
            missing.append(new_name)
    if missing:
        raise RuntimeError(f"keygen did not emit {missing}; stdout:\n{result.stdout}")
    return out


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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Binary upgrade test driver")
    parser.add_argument("--log-level", default="INFO")
    parser.add_argument(
        "--scenario",
        type=Scenario,
        choices=list(Scenario),
        default=Scenario.VANILLA,
    )
    parser.add_argument(
        "--pull-only",
        action="store_true",
        help="Pull base and (if UPGRADE_PULL=1) upgrade images, then exit.",
    )
    return parser.parse_args()


def run_scenario(compose: Compose, config: Config, scenario: Scenario) -> None:
    match scenario:
        case Scenario.VANILLA:
            compose.vanilla(config)
        case Scenario.NEW_FROM_OLD_FS:
            compose.new_from_old(config, WIPE_FS_NODE, compose.wipe_fs_node)
        case Scenario.NEW_FROM_OLD_PG:
            compose.new_from_old(config, WIPE_PG_NODE, compose.wipe_pg_node)
        case Scenario.OLD_FROM_NEW_FS:
            compose.old_from_new(config, NODE_5_FS_OVERLAY)
        case Scenario.OLD_FROM_NEW_PG:
            compose.old_from_new(config, NODE_5_PG_OVERLAY)


def main() -> int:
    args = parse_args()
    logging.basicConfig(
        level=args.log_level,
        format="%(levelname)s %(name)s: %(message)s",
        stream=sys.stderr,
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
        run_scenario(compose, config, args.scenario)
        log.info("Binary upgrade test complete")
    return 0


if __name__ == "__main__":
    sys.exit(main())
