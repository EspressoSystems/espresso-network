"""Test doubles and gates shared by the network-bench tests. Never imported by `netbench.py` or
`aws-bench`, which ship to hosts without this file."""

import asyncio
import base64
import json
import os
import threading
import time
import unittest
from collections.abc import Awaitable, Callable, Coroutine
from typing import Any, TypeVar
from urllib.parse import urlsplit

import netbench

SLOW = unittest.skipUnless(os.environ.get("SLOW_TESTS"), "SLOW_TESTS unset")

SINGLE_MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json"
INDEX_MEDIA_TYPE = "application/vnd.oci.image.index.v1+json"
JSON_MEDIA_TYPE = "application/json"

Reply = tuple[int, dict[str, str], bytes]


def _json_reply(obj: dict[str, Any], content_type: str = JSON_MEDIA_TYPE) -> Reply:
    return 200, {"Content-Type": content_type}, json.dumps(obj).encode()


class FakeRegistry:
    """A minimal OCI/Docker registry usable as the `fetch` argument of `resolve_image`: one
    repository and tag, anonymous token challenge. `platforms` is a list of `(os,
    architecture)` pairs for the index; a `tag` of `"missing"` makes the manifest request 404,
    `deny_token` makes the token endpoint 401 (a private image), `deny_manifest_status` makes
    the manifest request fail with that status without ever offering a token challenge, and
    `index=False` serves a single manifest at the tag instead of a multi-platform index."""

    def __init__(
        self,
        repository: str,
        tag: str,
        platforms: list[tuple[str, str]],
        revision: str | None = None,
        deny_token: bool = False,
        deny_manifest_status: int | None = None,
        index: bool = True,
        host: str = "registry.test",
    ):
        self.repository = repository
        self.tag = tag
        self.platforms = platforms
        self.revision = revision
        self.deny_token = deny_token
        self.deny_manifest_status = deny_manifest_status
        self.host = host
        self.digests = {p: f"sha256:{i:064d}" for i, p in enumerate(platforms)}
        self.config_digest = "sha256:" + "c" * 64
        manifest_prefix = f"/v2/{repository}/manifests/"
        single = _json_reply(
            {
                "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                "config": {"digest": self.config_digest},
            }
        )
        index_reply = _json_reply(
            {
                "mediaType": INDEX_MEDIA_TYPE,
                "manifests": [
                    {
                        "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                        "digest": self.digests[p],
                        "platform": {"os": p[0], "architecture": p[1]},
                    }
                    for p in platforms
                ],
            },
            content_type=INDEX_MEDIA_TYPE,
        )
        labels = (
            {"org.opencontainers.image.revision": revision}
            if revision is not None
            else {}
        )
        self.routes: dict[str, Reply] = {
            f"{manifest_prefix}{digest}": single for digest in self.digests.values()
        }
        if tag != "missing":
            self.routes[f"{manifest_prefix}{tag}"] = (
                single if not index else index_reply
            )
        self.routes[f"/v2/{repository}/blobs/{self.config_digest}"] = _json_reply(
            {"architecture": "arm64", "os": "linux", "config": {"Labels": labels}}
        )

    @property
    def ref(self) -> str:
        return f"{self.host}/{self.repository}:{self.tag}"

    def __call__(self, url: str, headers: dict[str, str]) -> Reply:
        path = url.split(self.host, 1)[1]
        if path.startswith("/token"):
            if self.deny_token:
                return 401, {}, b""
            return _json_reply({"token": "faketoken"})
        if f"/v2/{self.repository}/manifests/" in path:
            if self.deny_manifest_status is not None:
                return self.deny_manifest_status, {}, b""
            if headers.get("Authorization") != "Bearer faketoken":
                scheme = "http" if self.host.startswith("127.0.0.1:") else "https"
                challenge = (
                    f'Bearer realm="{scheme}://{self.host}/token",service="fake",'
                    f'scope="repository:{self.repository}:pull"'
                )
                return 401, {"WWW-Authenticate": challenge}, b""
        return self.routes.get(path, (404, {}, b""))


T = TypeVar("T")

BLOCK_S = 0.05
SPIN_S = 0.002


class FakeClock:
    """Virtual time for single-threaded code: a wait advances the clock instead of blocking.
    `sleeps` lists the durations of every wait that advanced it. `on_advance(now)` runs after
    each advance. Advancing past `limit_s` raises, so a poller that never reaches its exit
    condition fails instead of spinning."""

    def __init__(
        self,
        limit_s: float = 3600.0,
        on_advance: Callable[[float], None] | None = None,
    ) -> None:
        self.limit_s = limit_s
        self.on_advance = on_advance
        self.now = 0.0
        self.sleeps: list[float] = []

    def time(self) -> float:
        return self.now

    def monotonic(self) -> float:
        return self.now

    def sleep(self, s: float) -> None:
        self.sleeps.append(s)
        self.now += s
        if self.now > self.limit_s:
            raise RuntimeError(f"FakeClock: advanced past {self.limit_s:g} s")
        if self.on_advance is not None:
            self.on_advance(self.now)

    def wait(self, event: threading.Event, s: float) -> bool:
        if event.is_set():
            return True
        self.sleep(s)
        return event.is_set()

    async def asleep(self, s: float) -> None:
        self.sleep(s)
        await asyncio.sleep(0)

    async def wait_for(self, aw: Awaitable[T], s: float) -> T:
        """Times out: nothing else runs while a `FakeClock` advances."""
        if isinstance(aw, Coroutine):
            aw.close()
        await self.asleep(s)
        raise TimeoutError


class ScaledClock:
    """Real time made `scale` times faster: `sleep(s)` blocks `s / scale` wall seconds. For
    code whose threads and tasks must keep running concurrently."""

    def __init__(self, scale: float) -> None:
        self.scale = scale
        self.start = time.monotonic()
        self.epoch = time.time()

    def time(self) -> float:
        return self.epoch + (time.monotonic() - self.start) * self.scale

    def monotonic(self) -> float:
        return self.start + (time.monotonic() - self.start) * self.scale

    def sleep(self, s: float) -> None:
        time.sleep(s / self.scale)

    def wait(self, event: threading.Event, s: float) -> bool:
        return event.wait(s / self.scale)

    async def asleep(self, s: float) -> None:
        """`asyncio.sleep` wakes up to 1 ms late, `scale` ms of clock time, which throttles a
        pacer whose interval is a few of those. The last SPIN_S wall seconds are spun."""
        deadline = self.monotonic() + s
        await asyncio.sleep(max(0.0, s / self.scale - SPIN_S))
        while self.monotonic() < deadline:
            await asyncio.sleep(0)

    async def wait_for(self, aw: Awaitable[T], s: float) -> T:
        return await asyncio.wait_for(aw, s / self.scale)


class FakePool:
    """One client's view of a `FakeNode`, with the `closed` flag of its own."""

    def __init__(self, node: "FakeNode", clock: netbench.Clock) -> None:
        self.node = node
        self.clock = clock
        self.closed = threading.Event()

    def request(
        self, method: str, url: str, body: bytes | None = None, timeout: float = 10.0
    ) -> tuple[int, bytes]:
        return self.node.request(method, url, body)

    def close(self) -> None:
        self.closed.set()


class FakeNode:
    """Submit, block height and payload endpoints, in process; `include` decides whether blocks
    carry the submitted transactions. A submit takes `accept_delay` s before the transaction
    is taken and `reply_delay` s after. Payloads in `lost` are never served, those in `late`
    only `late[height]` s after their block was made; every payload answer takes
    `payload_delay` s. The query API shows a block `query_lag` s after the validator status
    API. A block takes at most `block_txs` transactions. Delays run on the calling thread.
    A block is made every BLOCK_S clock seconds, when a request finds it due."""

    def __init__(
        self,
        clock: netbench.Clock,
        include: bool,
        lost: frozenset[int] = frozenset(),
        accept_delay: float = 0.0,
        reply_delay: float = 0.0,
        late: dict[int, float] | None = None,
        payload_delay: float = 0.0,
        query_lag: float = 0.0,
        block_txs: int | None = None,
    ) -> None:
        self.clock = clock
        self.include = include
        self.lost = lost
        self.late = late or {}
        self.accept_delay = accept_delay
        self.reply_delay = reply_delay
        self.payload_delay = payload_delay
        self.query_lag = query_lag
        self.block_txs = block_txs
        self.lock = threading.Lock()
        self.pending: list[bytes] = []
        self.blocks = [b""]
        self.made = [clock.time()]
        self.submits: list[float] = []
        self.max_outstanding = 0

    @property
    def url(self) -> str:
        return "http://fake-node"

    def connect(self, clock: netbench.Clock) -> FakePool:
        return FakePool(self, clock)

    def request(
        self, method: str, url: str, body: bytes | None = None
    ) -> tuple[int, bytes]:
        path = urlsplit(url).path
        if method == "POST" and path == "/v1/submit/submit":
            assert body is not None
            return self.submit(body)
        if method == "GET":
            return self.get(path)
        raise ValueError(f"FakeNode: unexpected {method} {path}")

    def submit(self, body: bytes) -> tuple[int, bytes]:
        tx = json.loads(body)
        self.clock.sleep(self.accept_delay)
        with self.lock:
            self.produce()
            self.pending.append(base64.b64decode(tx["payload"]))
            self.submits.append(self.clock.time())
            self.max_outstanding = max(self.max_outstanding, len(self.pending))
        self.clock.sleep(self.reply_delay)
        return 200, json.dumps("TX~fake").encode()

    def get(self, path: str) -> tuple[int, bytes]:
        prefix = "/v1/availability/payload/"
        if path.startswith(prefix):
            self.clock.sleep(self.payload_delay)
        with self.lock:
            self.produce()
            if path == "/v1/node/block-height":
                shown = self.clock.time() - self.query_lag
                return 200, json.dumps(sum(1 for t in self.made if t <= shown)).encode()
            if path == "/v1/status/block-height":
                return 200, json.dumps(len(self.blocks) - 1).encode()
            if path == "/v1/node/stake-table/current":
                return 200, json.dumps({"stake_table": []}).encode()
            if path == "/v1/status/metrics":
                decided = sum(len(block) for block in self.blocks)
                return (
                    200,
                    f"consensus_finalized_bytes_sum {decided}\n"
                    "consensus_number_of_timeouts 0\n".encode(),
                )
            height = int(path.removeprefix(prefix))
            if height >= len(self.blocks) or height in self.lost:
                return 404, json.dumps("not found").encode()
            if self.clock.time() - self.made[height] < self.late.get(height, 0.0):
                return 404, json.dumps("not found").encode()
            raw = base64.b64encode(self.blocks[height]).decode()
        payload = {"data": {"raw_payload": raw, "ns_table": {"bytes": ""}}}
        return 200, json.dumps(payload).encode()

    def produce(self) -> None:
        """Makes the blocks due since the last request. Callers hold `lock`."""
        while self.made[-1] + BLOCK_S <= self.clock.time():
            taken = self.pending[: self.block_txs] if self.include else []
            if self.include:
                self.pending = self.pending[len(taken) :]
            self.blocks.append(b"".join(taken))
            self.made.append(self.made[-1] + BLOCK_S)
