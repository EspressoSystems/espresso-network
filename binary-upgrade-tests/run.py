#!/usr/bin/env python3
"""Binary upgrade test driver.

Boots docker-compose using docker-compose.yaml + .env from the BASE_TAG git
revision, then rolls each espresso-node-N from BASE_TAG to UPGRADE_TAG one
by one, bulk-upgrades the rest, and runs scripts/smoke-test-demo before and
after.
"""

from __future__ import annotations

import argparse
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
from dataclasses import dataclass
from pathlib import Path

log = logging.getLogger("binary-upgrade-test")

NODE_INDICES = (0, 1, 2, 3, 4)

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
    "wait-for-v4",
)

ESPRESSO_IMAGE_PREFIX = "ghcr.io/espressosystems/espresso-network/"

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)


@dataclass(frozen=True)
class Config:
    base_tag: str
    upgrade_tag: str
    keep_running: bool
    upgrade_pull: bool

    @classmethod
    def from_env(cls) -> Config:
        return cls(
            base_tag=os.environ.get("BASE_TAG", "20260505"),
            upgrade_tag=os.environ.get("UPGRADE_TAG", "main"),
            keep_running=os.environ.get("KEEP_RUNNING") == "1",
            upgrade_pull=os.environ.get("UPGRADE_PULL") == "1",
        )


@dataclass(frozen=True)
class Compose:
    base_dir: Path  # holds the extracted docker-compose.yaml + .env

    @property
    def base_args(self) -> list[str]:
        return [
            "docker", "compose",
            "--project-directory", str(REPO_ROOT),
            "--env-file", str(self.base_dir / ".env"),
            "-f", str(self.base_dir / "docker-compose.yaml"),
            "-f", str(REPO_ROOT / "binary-upgrade-tests" / "compose.persist-storage.yaml"),
        ]  # fmt: skip

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


def _http_status_and_body(url: str, timeout: float = 5.0) -> tuple[int, str]:
    try:
        with urllib.request.urlopen(url, timeout=timeout) as resp:
            return resp.status, resp.read().decode().strip()
    except urllib.error.HTTPError as e:
        return e.code, ""
    except (urllib.error.URLError, TimeoutError, ConnectionError, OSError):
        return 0, ""


def _height_at(api_url: str, path: str) -> int | None:
    code, body = _http_status_and_body(f"{api_url}{path}")
    return int(body) if code == 200 and body.isdigit() else None


def poll_until(
    check: Callable[[], bool], desc: str, timeout: float, interval: float = 2.0
) -> None:
    deadline = time.monotonic() + timeout
    while True:
        if check():
            return
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
        self, target: int, height_timeout: float = 120, leaf_timeout: float = 30
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

        poll_until(height_ok, f"{self} {name} >= {target}", height_timeout)

        if self.has_query:
            idx = target - 1

            def leaf_ok() -> bool:
                if self.leaf_available(idx):
                    log.info(f"{self} availability/leaf/{idx} ok")
                    return True
                return False

            poll_until(leaf_ok, f"{self} availability/leaf/{idx}", leaf_timeout)


def upgraded_services(compose: Compose) -> list[str]:
    return [s for s in compose.services() if s not in NOUPGRADE_SERVICES]


def assert_all_espresso_images(compose: Compose, expected_tag: str) -> None:
    bad: list[str] = []
    for service in upgraded_services(compose):
        cid = compose.container_id(service)
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


def roll_node(compose: Compose, n: int, upgrade_tag: str) -> None:
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
    compose.run(
        "up",
        "-d",
        "--no-deps",
        "--force-recreate",
        str(nodes[n]),
        docker_tag=upgrade_tag,
    )

    for node in nodes:
        node.wait_until_at_height(target)


def bulk_upgrade_remaining(compose: Compose, upgrade_tag: str) -> None:
    nodes = tuple(f"espresso-node-{i}" for i in NODE_INDICES)
    services = [s for s in upgraded_services(compose) if s not in nodes]
    if not services:
        raise RuntimeError("No remaining services to upgrade")
    log.info(
        f"Bulk-upgrading {len(services)} services to {upgrade_tag}: {' '.join(services)}"
    )
    compose.run("up", "-d", "--no-deps", *services, docker_tag=upgrade_tag)


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


def smoke_test(tag: str, base_dir: Path) -> None:
    # cwd=base_dir so `source .env` in the script picks up base_tag's .env;
    # deployed contract addresses match it, not REPO_ROOT/.env which may
    # have shifted if main changed deploy ordering.
    subprocess.run(
        ["timeout", "600", str(REPO_ROOT / "scripts" / "smoke-test-demo")],
        cwd=base_dir,
        env=os.environ | {"DOCKER_TAG": tag},
        check=True,
    )


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
        "--pull-only",
        action="store_true",
        help="Pull base and (if UPGRADE_PULL=1) upgrade images, then exit.",
    )
    return parser.parse_args()


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
    os.environ.setdefault(
        "ESPRESSO_SEQUENCER_GENESIS_FILE", "genesis/demo-drb-header.toml"
    )
    os.environ.setdefault("ESPRESSO_NODE_GENESIS_FILE", "genesis/demo-drb-header.toml")
    load_project_env()

    with compose_session(config) as compose:
        log.info(f"Pulling base images (DOCKER_TAG={config.base_tag})")
        compose.run("pull", "--policy", "missing", docker_tag=config.base_tag)
        if config.upgrade_pull:
            log.info(f"Pulling upgrade images (DOCKER_TAG={config.upgrade_tag})")
            compose.run("pull", "--policy", "missing", docker_tag=config.upgrade_tag)

        if args.pull_only:
            log.info("--pull-only: images pulled, exiting before stack start")
            return 0

        # Preflight: clean any stale stack.
        compose.run("down", "-v", check=False, capture=True)

        log.info(f"Starting network on {config.base_tag}")
        # `compose up -d` blocks on `depends_on: service_completed_successfully`,
        # but `deploy-lcv3-upgrade` retries forever and `prover-one-shot` has a
        # broken healthcheck, so a synchronous call would never return. Run in
        # the background and let the smoke test below verify readiness end to
        # end. The compose stack is torn down on context exit regardless.
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

        log.info("Initial smoke test")
        smoke_test(config.base_tag, compose.base_dir)

        for n in NODE_INDICES:
            log.info(f"Rolling espresso-node-{n} to {config.upgrade_tag}")
            roll_node(compose, n, config.upgrade_tag)

        log.info(f"Bulk-upgrading remaining services to {config.upgrade_tag}")
        bulk_upgrade_remaining(compose, config.upgrade_tag)

        log.info(f"Asserting all espresso-network images run tag {config.upgrade_tag}")
        assert_all_espresso_images(compose, config.upgrade_tag)

        log.info("Final smoke test")
        smoke_test(config.upgrade_tag, compose.base_dir)

        log.info("Binary upgrade test complete")
    return 0


if __name__ == "__main__":
    sys.exit(main())
