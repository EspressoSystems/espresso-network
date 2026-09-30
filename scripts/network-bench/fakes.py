"""Test doubles and helpers shared by the network-bench tests. Never imported by `netbench.py`
or `aws-bench`, which ship to hosts without this file."""

import asyncio
import base64
import bisect
import collections
import contextlib
import dataclasses
import heapq
import importlib.util
import io
import itertools
import json
import math
import selectors
import shutil
import signal
import stat
import subprocess
import tempfile
import threading
import unittest
import unittest.mock
from collections.abc import Awaitable, Callable, Coroutine, Iterable
from dataclasses import dataclass, field
from datetime import UTC, datetime
from importlib.machinery import SourceFileLoader
from pathlib import Path
from typing import Any, ClassVar, TypeVar
from urllib.parse import urlsplit

import netbench

SINGLE_MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json"
INDEX_MEDIA_TYPE = "application/vnd.oci.image.index.v1+json"
JSON_MEDIA_TYPE = "application/json"

Reply = tuple[int, dict[str, str], bytes]

FAKE_EPOCH = datetime(2026, 9, 29, 16, 0, tzinfo=UTC).timestamp()
# Mirrors `aws-bench`'s CHECKIP_URL.
CHECKIP_URL = "https://checkip.amazonaws.com"


def _json_reply(obj: dict[str, Any], content_type: str = JSON_MEDIA_TYPE) -> Reply:
    return 200, {"Content-Type": content_type}, json.dumps(obj).encode()


class FakeRegistry:
    """A minimal OCI/Docker registry usable as `FakeSystem.http`: one repository and tag,
    anonymous token challenge on manifests. `platforms` is a list of `(os, architecture)` pairs
    for the index; a `tag` of `"missing"` makes the manifest request 404. `routes` maps a path
    to its reply."""

    host = "registry.test"

    def __init__(
        self,
        repository: str,
        tag: str,
        platforms: list[tuple[str, str]],
        revision: str | None = None,
    ):
        self.repository = repository
        self.tag = tag
        self.platforms = platforms
        self.revision = revision
        self.digests = {p: f"sha256:{i:064d}" for i, p in enumerate(platforms)}
        self.config_digest = "sha256:" + "c" * 64
        self.single_digest = "sha256:" + "d" * 64
        self.manifest_prefix = f"/v2/{repository}/manifests/"
        status, headers, body = _json_reply(
            {
                "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                "config": {"digest": self.config_digest},
            }
        )
        self.single: Reply = (
            status,
            {**headers, "Docker-Content-Digest": self.single_digest},
            body,
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
            f"{self.manifest_prefix}{digest}": self.single
            for digest in self.digests.values()
        }
        if tag != "missing":
            self.routes[f"{self.manifest_prefix}{tag}"] = index_reply
        self.routes[f"/v2/{repository}/blobs/{self.config_digest}"] = _json_reply(
            {"architecture": "arm64", "os": "linux", "config": {"Labels": labels}}
        )
        self.routes["/token"] = _json_reply({"token": "faketoken"})

    @property
    def ref(self) -> str:
        return f"{self.host}/{self.repository}:{self.tag}"

    def __call__(self, url: str, headers: dict[str, str]) -> Reply:
        parts = urlsplit(url)
        if parts.netloc != self.host:
            raise AssertionError(f"FakeRegistry: unexpected host in {url}")
        authorized = headers.get("Authorization") == "Bearer faketoken"
        if parts.path.startswith(self.manifest_prefix) and not authorized:
            challenge = (
                f'Bearer realm="https://{self.host}/token",service="fake",'
                f'scope="repository:{self.repository}:pull"'
            )
            return 401, {"WWW-Authenticate": challenge}, b""
        return self.routes.get(parts.path, (404, {}, b""))


T = TypeVar("T")

BLOCK_S = 0.05


class FakeClock:
    """Virtual time: a wait advances the clock instead of blocking. `sleeps` lists the
    durations of every wait. `on_advance(now)` runs after each advance. Advancing past
    `limit_s` raises, so a poller that never reaches its exit condition fails instead of
    spinning. `run` drives a coroutine on an event loop whose timers run on this clock; outside
    `run` a wait advances the clock at once. Inside `run`, executor jobs run at once on the loop
    thread, or with `threaded` on executor threads that may wait on this clock."""

    def __init__(
        self,
        limit_s: float = 3600.0,
        on_advance: Callable[[float], None] | None = None,
        start: float = 0.0,
        threaded: bool = False,
    ) -> None:
        self.threaded = threaded
        self.limit_s = limit_s
        self.on_advance = on_advance
        self.now = start
        self.start = start
        self.sleeps: list[float] = []
        self.sched: _Scheduler | None = None

    def time(self) -> float:
        return self.now

    def monotonic(self) -> float:
        return self.now

    def sleep(self, s: float) -> None:
        self.sleeps.append(s)
        if self.sched is None:
            self.advance(self.now + s)
        elif s > 0:
            self.sched.sleep(self.now + s)

    def wait(self, event: threading.Event, s: float) -> bool:
        if event.is_set():
            return True
        self.sleep(s)
        return event.is_set()

    async def asleep(self, s: float) -> None:
        if self.sched is None:
            raise AssertionError("FakeClock: asleep outside FakeClock.run")
        self.sleeps.append(s)
        await asyncio.sleep(s)

    async def wait_for(self, aw: Awaitable[T], s: float) -> T:
        return await asyncio.wait_for(aw, s)

    def run(self, main: Coroutine[Any, Any, T]) -> T:
        if self.sched is not None:
            raise RuntimeError("FakeClock: run is not reentrant")
        self.sched = _Scheduler(self)
        try:
            with asyncio.Runner(loop_factory=self.sched.new_loop) as runner:
                return runner.run(main)
        finally:
            self.sched = None

    def advance(self, to: float) -> None:
        self.now = to
        if self.now - self.start > self.limit_s:
            raise RuntimeError(f"FakeClock: advanced past {self.limit_s:g} s")
        if self.on_advance is not None:
            self.on_advance(self.now)


class _Closed(BaseException):
    """Unwinds a worker left blocked when its event loop closes."""


class _Worker:
    """One executor thread. It runs only while it holds the turn (its `go` lock released), so
    the event loop and the workers take turns and the clock moves only when all of them wait."""

    def __init__(self, sched: "_Scheduler", pool: "_Pool") -> None:
        self.sched = sched
        self.pool = pool
        self.jobs: collections.deque[_Job] = collections.deque()
        self.go = threading.Lock()
        self.go.acquire()
        threading.Thread(target=self.main, daemon=True).start()

    def main(self) -> None:
        self.sched.local.worker = self
        try:
            self.wait_turn()
            while True:
                while self.jobs:
                    self.jobs.popleft().run(self.sched)
                    if self.pool.queue:
                        self.jobs.append(self.pool.queue.popleft())
                self.pool.idle.append(self)
                self.yield_turn()
        except _Closed:
            pass

    def wait_turn(self) -> None:
        self.go.acquire()
        if self.sched.closed:
            raise _Closed

    def yield_turn(self) -> None:
        self.sched.loop_go.release()
        self.wait_turn()


@dataclass
class _Job:
    fn: Callable[..., Any]
    args: tuple[Any, ...]
    future: asyncio.Future

    def run(self, sched: "_Scheduler") -> None:
        try:
            result, error = self.fn(*self.args), None
        except Exception as exc:  # noqa: BLE001
            result, error = None, exc
        sched.settled.append((self.future, result, error))


def _settle(future: asyncio.Future, result: Any, error: Exception | None) -> None:
    if future.cancelled():
        return
    if error is None:
        future.set_result(result)
    else:
        future.set_exception(error)


@dataclass
class _Pool:
    """An executor's workers; `max_workers` of them at most, as the real executor has."""

    max_workers: int
    workers: list[_Worker] = field(default_factory=list)
    idle: list[_Worker] = field(default_factory=list)
    queue: collections.deque[_Job] = field(default_factory=collections.deque)


class _Scheduler:
    """Hands the turn between the event loop thread and the executor workers. The loop's
    selector advances the clock to the next timer or sleeping worker once nothing can run."""

    def __init__(self, clock: FakeClock) -> None:
        self.clock = clock
        self.loop_go = threading.Lock()
        self.loop_go.acquire()
        self.local = threading.local()
        self.closed = False
        self.pools: dict[int, _Pool] = {}
        self.runnable: collections.deque[_Worker] = collections.deque()
        self.settled: list[tuple[asyncio.Future, Any, Exception | None]] = []
        self.sleepers: list[tuple[float, int, _Worker]] = []
        self.seq = itertools.count()

    def new_loop(self) -> asyncio.AbstractEventLoop:
        self.loop = _VirtualLoop(self)
        return self.loop

    def submit(
        self, executor: Any, fn: Callable[..., Any], args: tuple
    ) -> asyncio.Future:
        future = self.loop.create_future()
        job = _Job(fn, args, future)
        if not self.clock.threaded:
            job.run(self)
            self.run_workers()
            return future
        pool = self.pools.setdefault(id(executor), _Pool(executor._max_workers))
        if pool.idle:
            worker = pool.idle.pop()
        elif len(pool.workers) < pool.max_workers:
            worker = _Worker(self, pool)
            pool.workers.append(worker)
        else:
            pool.queue.append(job)
            return future
        worker.jobs.append(job)
        self.runnable.append(worker)
        return future

    def sleep(self, until: float) -> None:
        worker: _Worker | None = getattr(self.local, "worker", None)
        if worker is None:
            raise AssertionError(
                "FakeClock: blocking sleep on the event loop thread; needs threaded=True"
            )
        heapq.heappush(self.sleepers, (until, next(self.seq), worker))
        worker.yield_turn()

    def run_workers(self) -> bool:
        """True if a job finished; its future settles on the next loop iteration."""
        while self.runnable:
            self.runnable.popleft().go.release()
            self.loop_go.acquire()
        settled, self.settled = self.settled, []
        for args in settled:
            self.loop.call_soon(_settle, *args)
        return bool(settled)

    def select(
        self, poll: Callable[[float | None], list], timeout: float | None
    ) -> list:
        while True:
            settled = self.run_workers()
            events = poll(0)
            if events or settled or timeout == 0:
                return events
            due = math.inf if timeout is None else self.clock.now + timeout
            wake = self.sleepers[0][0] if self.sleepers else math.inf
            if wake == due == math.inf:
                raise RuntimeError("FakeClock: nothing left to wait for")
            if wake <= due:
                _, _, worker = heapq.heappop(self.sleepers)
                self.clock.advance(max(wake, self.clock.now))
                self.runnable.append(worker)
                timeout = None if timeout is None else due - self.clock.now
                continue
            self.clock.advance(due)
            return []

    def close(self) -> None:
        self.closed = True
        for pool in self.pools.values():
            for worker in pool.workers:
                worker.go.release()


class _VirtualSelector(selectors.DefaultSelector):
    def __init__(self, sched: _Scheduler) -> None:
        super().__init__()
        self.sched = sched

    def select(self, timeout: float | None = None) -> list:
        return self.sched.select(super().select, timeout)


class _VirtualLoop(asyncio.SelectorEventLoop):
    def __init__(self, sched: _Scheduler) -> None:
        super().__init__(_VirtualSelector(sched))
        self.sched = sched

    def time(self) -> float:
        return self.sched.clock.now

    def run_in_executor(self, executor: Any, func: Callable[..., T], *args: Any):
        if executor is None:
            raise AssertionError("FakeClock: the default executor has no size")
        return self.sched.submit(executor, func, args)

    def close(self) -> None:
        self.sched.close()
        super().close()


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
        self.decided = 0
        self.made = [clock.time()]
        self.submits: list[float] = []

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
                return 200, json.dumps(bisect.bisect_right(self.made, shown)).encode()
            if path == "/v1/status/block-height":
                return 200, json.dumps(len(self.blocks) - 1).encode()
            if path == "/v1/node/stake-table/current":
                return 200, json.dumps({"stake_table": []}).encode()
            if path == "/v1/status/metrics":
                return (
                    200,
                    f"consensus_finalized_bytes_sum {self.decided}\n"
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
            self.decided += len(self.blocks[-1])
            self.made.append(self.made[-1] + BLOCK_S)


def completed(
    stdout: str = "", returncode: int = 0, stderr: str = ""
) -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess(
        args=[], returncode=returncode, stdout=stdout, stderr=stderr
    )


def fake_ssh_keygen(argv: list[str]) -> subprocess.CompletedProcess:
    key = Path(argv[argv.index("-f") + 1])
    key.write_text("private")
    key.chmod(0o600)
    key.with_name(key.name + ".pub").write_text("ssh-ed25519 GENERATED run\n")
    return completed()


def host_info(name: str, role: str, index: int) -> dict:
    return {
        "name": name,
        "role": role,
        "public_ip": f"203.0.113.{index}",
        "private_ip": f"10.0.0.{index}",
        "private_dns": f"ip-10-0-0-{index}.eu-west-1.compute.internal",
        "instance_id": f"i-{index:012x}",
    }


def two_node_hosts_info() -> dict:
    return {
        "ctl": host_info("ctl", "ctl", 1),
        "node0": host_info("node0", "query", 2),
        "node1": host_info("node1", "validator", 3),
    }


def metric_data(
    byte: list[float | None], io: list[float | None], first: float = 60.0
) -> str:
    """`cloudwatch get-metric-data` output; one period per list item starting at `first`, a
    None item is a period without datapoint."""

    def result(query_id: str, values: list[float | None]) -> dict:
        points = [
            (datetime.fromtimestamp(first + 60.0 * i, UTC).isoformat(), v)
            for i, v in enumerate(values)
            if v is not None
        ]
        return {
            "Id": query_id,
            "Label": query_id,
            "Timestamps": [ts for ts, _ in points],
            "Values": [v for _, v in points],
            "StatusCode": "Complete",
        }

    return json.dumps(
        {"MetricDataResults": [result("byte", byte), result("io", io)], "Messages": []}
    )


FULL_BALANCE = metric_data([100.0, 100.0, 100.0], [100.0, 100.0, 100.0])


FLEET_ARNS = [
    "arn:aws:ec2:eu-west-1:1:instance/i-1",
    "arn:aws:ec2:eu-west-1:1:security-group/sg-1",
    "arn:aws:ec2:eu-west-1:1:key-pair/key-1",
]


class FakeRunner:
    """Records every call and answers it from the `respond` table, then from `responses` (argv
    prefix to reply). A fleet runner (`states` given) answers the rest as a 2-node fleet would
    to `cmd_run` (git, tofu, ssh, rsync, aws); `states` are the successive agent-state.json
    contents, the last one repeating. Without `states` an unmatched argv raises, so a test can
    prove a refused plan never reached `aws` or `tofu`."""

    ready_digests: ClassVar[dict[str, str]]
    """Image name to `ref@digest` as `docker image inspect` reports it on a host."""

    def __init__(
        self,
        responses: dict[tuple[str, ...], subprocess.CompletedProcess] | None = None,
        states: list[dict] | None = None,
        apply: subprocess.CompletedProcess | None = None,
        destroys: list[subprocess.CompletedProcess] | None = None,
        on_poll=None,
        describe: str = "[]",
        balance: subprocess.CompletedProcess | None = None,
    ):
        self.responses = responses or {}
        self.states = states
        self.apply = apply or completed()
        self.destroys = destroys or [completed()]
        self.on_poll = on_poll
        self.describe = describe
        self.balance = balance or completed(stdout=FULL_BALANCE)
        self.polls = 0
        self.calls: list[list[str]] = []
        self.envs: list[dict[str, str] | None] = []
        self.lock = threading.Lock()
        self.table: list[
            tuple[str, Callable[[list[str]], subprocess.CompletedProcess]]
        ] = []

    def respond(
        self, pattern: str, reply: Callable[[list[str]], subprocess.CompletedProcess]
    ) -> None:
        """`reply(argv)` answers every argv whose space-joined form contains `pattern`. The first
        pattern added that matches wins."""
        self.table.append((pattern, reply))

    def ran(self, *needle: str) -> bool:
        return any(all(n in " ".join(call) for n in needle) for call in self.calls)

    def count(self, *needle: str) -> int:
        return sum(all(n in " ".join(c) for n in needle) for c in self.calls)

    def __call__(
        self, argv: list[str], env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess:
        with self.lock:
            self.calls.append(argv)
            self.envs.append(env)
        joined = " ".join(argv)
        for pattern, reply in self.table:
            if pattern in joined:
                return reply(argv)
        if argv[0] == "ssh-keygen":
            return fake_ssh_keygen(argv)
        for prefix, result in self.responses.items():
            if tuple(argv[: len(prefix)]) == prefix:
                return result
        if self.states is None:
            raise AssertionError(f"unexpected command: {argv!r}")
        return self.default(argv)

    def default(self, argv: list[str]) -> subprocess.CompletedProcess:
        if self.states is None:
            raise AssertionError("FakeRunner: `default` needs `states`")
        if argv[0] == "git":
            return completed(stdout="a" * 40 + "\n")
        if argv[0] == "tofu":
            return self.tofu(argv[2])
        if argv[0] == "ssh":
            return self.ssh(argv[-1])
        if argv[0] == "rsync":
            return completed()
        return self.aws(argv)

    def tofu(self, verb: str) -> subprocess.CompletedProcess:
        if verb == "apply":
            return self.apply
        if verb == "output":
            hosts = two_node_hosts_info()
            return completed(stdout=json.dumps({"hosts": {"value": hosts}}))
        if verb == "destroy":
            with self.lock:
                return (
                    self.destroys.pop(0) if len(self.destroys) > 1 else self.destroys[0]
                )
        return completed(stdout="plan")

    def ssh(self, command: str) -> subprocess.CompletedProcess:
        if "cloud-init status --format json" in command:
            return completed(stdout=json.dumps({"status": "done"}))
        if "ready.json" in command:
            tracking = "System time     : 0.000001000 seconds fast of NTP time\n"
            return completed(
                stdout=json.dumps(
                    {"digests": self.ready_digests, "chronyc_tracking": tracking}
                )
            )
        if "agent-state.json" in command:
            assert self.states is not None
            with self.lock:
                index = min(self.polls, len(self.states) - 1)
                self.polls += 1
            if self.on_poll:
                self.on_poll(self.polls)
            return completed(stdout=json.dumps(self.states[index]))
        if "date +%s.%N" in command:
            return completed(stdout="1000.5\n")
        if "docker inspect -f" in command:
            return completed(stdout="2026-09-29T15:00:00.100000000Z\n")
        if "docker wait deploy" in command:
            return completed(stdout="0\n")
        return completed()

    def aws(self, argv: list[str]) -> subprocess.CompletedProcess:
        if "describe-instances" in argv:
            return completed(stdout=self.describe)
        if "get-metric-data" in argv:
            return self.balance
        if "get-resources" in argv:
            return completed(stdout=json.dumps(FLEET_ARNS))
        return completed()


def no_http_pool(clock: netbench.Clock) -> netbench.Http:
    raise AssertionError("unexpected HTTP pool")


@dataclass
class FakeSystem:
    """Structural twin of `aws-bench`'s `System`, which this file cannot import."""

    run: Callable[..., subprocess.CompletedProcess[str]] = field(
        default_factory=FakeRunner
    )
    clock: netbench.Clock = field(default_factory=lambda: FakeClock(start=FAKE_EPOCH))
    handlers: dict[int, Callable[[int, Any], None]] = field(default_factory=dict)
    tools: set[str] | None = None
    answer: bool = False
    prompts: list[str] = field(default_factory=list)
    http: Callable[[str, dict[str, str]], Reply] | None = None
    http_pool: Callable[[netbench.Clock], netbench.Http] = no_http_pool
    user: Callable[[], str] = lambda: "tester"
    hostname: Callable[[], str] = lambda: "testhost"
    pid: int = 4242
    dead_pids: set[int] = field(default_factory=set)

    def which(self, name: str) -> str | None:
        if self.tools is None or name in self.tools:
            return f"/usr/bin/{name}"
        return None

    def ask(self, prompt: str) -> bool:
        self.prompts.append(prompt)
        return self.answer

    def http_get(self, url: str, headers: dict[str, str]) -> Reply:
        if url == CHECKIP_URL:
            return 200, {}, b"203.0.113.5\n"
        if self.http is not None:
            return self.http(url, headers)
        raise AssertionError(f"unexpected http GET {url}")

    def trap(
        self,
        signals: tuple[signal.Signals, ...],
        handler: Callable[[int, Any], None],
    ) -> None:
        for sig in signals:
            self.handlers[sig] = handler

    def pid_alive(self, pid: int) -> bool:
        return pid not in self.dead_pids

    def fire(self, signum: int) -> None:
        self.handlers[signum](signum, None)


TOPOLOGY: netbench.Topology = {
    "nodes": {
        "node0": "http://localhost:24000",
        "node1": "http://localhost:24001",
        "node2": "http://localhost:24002",
    },
    "roles": {
        "node0": "validator, sqlite",
        "node1": "validator, sqlite",
        "node2": "validator, sqlite",
    },
    "query_node": "node0",
}


def quantiles(p50, p99):
    return {"n": 100, "mean": p50, "p50": p50, "p95": p99, "p99": p99, "max": p99}


def node(cpu):
    return {
        "role": "validator, sqlite",
        "decided_height_end": 500,
        "decided_blocks": 400,
        "view_lag_end": 0,
        "cpu_cores": cpu,
        "rss_peak_bytes": 500_000_000,
        "tokio_busy_frac": 0.2,
        "ops": {
            "consensus_storage_append_da": {
                "per_s": 2.0,
                "mean_ms": 5.0,
                "p50_ms": 4.0,
                "p99_ms": 20.0,
                "busy_frac": 0.01,
            }
        },
    }


def step(rate, decided=None, consensus=(), query=(), consensus_p50=900.0):
    """A step at `rate` MB/s that decides `decided` (default: all of it)."""
    return {
        "rate_mb_s": rate,
        "refine": False,
        "t_start": 0.0,
        "t_mid": 15.0,
        "t_end": 30.0,
        "submitted_mb_s": rate,
        "decided_mb_s": rate if decided is None else decided,
        "timeouts": 0,
        "consensus_latency_ms": quantiles(consensus_p50, 2000.0),
        "query_lag_ms": quantiles(200.0, 400.0),
        "query_lag_slope_ms_s": 0.0,
        "latency_ms": quantiles(1200.0, 2500.0),
        "mean_view_ms": 500.0,
        "cpu_s_per_mb": 0.5,
        "node_cpu": {"node0": 1.0, "node1": 0.5, "node2": 0.5},
        "postgres_cpu": 0.3,
        "consensus_fails": list(consensus),
        "query_fails": list(query),
        "passed": not consensus and not query,
    }


def make_result(steps=None, steal=0.0, config_hash="abc123") -> netbench.BenchResult:
    """Steps at 4, 6 and 8 MB/s, the last failing on decided: capacity 6 MB/s."""
    if steps is None:
        steps = [
            step(4.0),
            step(6.0),
            step(8.0, 7.0, consensus=["decided 88% of offered"]),
        ]
    result: netbench.BenchResult = {
        "schema_version": netbench.SCHEMA_VERSION,
        "run": {
            "sha": "0123456789abcdef",
            "ref": "refs/heads/main",
            "event": "push",
            "run_id": "1",
            "run_url": "https://github.com/o/r/actions/runs/1",
            "pr": None,
            "started_at": "2026-01-01T00:00:00+00:00",
            "wall_s": 400.0,
            "ready_s": 40.0,
            "local": False,
            "teardown": [],
        },
        "runner": {
            "cpu_model": "Test CPU",
            "nproc": 4,
            "affinity": 4,
            "cgroup_cpu_max": None,
            "mhz": [3000.0],
            "flags": [],
            "mem_total_bytes": 16_000_000_000,
            "kernel": "6",
            "image_os": "ubuntu24",
            "image_version": "1",
            "runner_name": "r",
        },
        "calibration": {
            "before": {
                "sha256_1t_mb_s": 2000.0,
                "sha256_mt_mb_s": 8000.0,
                "fsync_per_s": 300.0,
            },
            "after": {
                "sha256_1t_mb_s": 2000.0,
                "sha256_mt_mb_s": 8000.0,
                "fsync_per_s": 300.0,
            },
            "drift_pct": 0.0,
        },
        "config": {
            "workers": 6,
            "tx_size": 100_000,
            "submit_nodes": 3,
            "steps": [4.0, 6.0, 8.0],
            "step_s": 30,
            "warmup_s": 60,
            "cap_s": 5.0,
            "latency_target_ms": 1000,
            "query_lag_target_ms": 1000,
            "keep_going": False,
        },
        "config_hash": config_hash,
        "window": {"t0": 0.0, "t1": 180.0, "height_start": 100, "height_end": 500},
        "steps": steps,
        "capacity": netbench.capacity(steps),
        "nodes": {"node0": node(1.0), "node1": node(0.5), "node2": node(0.5)},
        "processes": {
            "postgres": {"cpu_cores_mean": 0.3, "cpu_s": 54.0, "rss_peak_bytes": 10}
        },
        "host": {
            "util_mean": 0.5,
            "util_max": 0.7,
            "steal_pct": steal,
            "iowait_pct": 1.0,
            "mem_avail_min_bytes": 8_000_000_000,
        },
        "load": {
            "submitted": 1000,
            "included": 990,
            "timeouts": 10,
            "submit_errors": 0,
            "max_in_flight": 48,
            "cap_waits": 0,
            "missing_payloads": [],
            "tracker_lag_ms": quantiles(50.0, 100.0),
            "drain_s": 3.0,
            "refine_skipped": False,
        },
        "stake_table": ["0x1", "0x1", "0x1"],
        "validity": {"valid": True, "noisy": False, "reasons": []},
    }
    result["validity"] = netbench.check_validity(
        result, {n: 1.0 for n in TOPOLOGY["nodes"]}
    )
    return result


def write_run_dir(out):
    """Steps at 1 MB/s (t 100 to 130) and 2 MB/s (130 to 160). Every node decides 1 MB/s in 2
    blocks/s and 4 views/s and uses 0.5 cores, the host is half busy, and a 1 MB transaction
    goes out every second from 110 to 150 (one times out): its block shows on a validator
    after 0.5 s and on node0 after 0.8 s, and is scanned 1.1 s after that."""
    t0, t1 = 100.0, 160.0

    def jsonl(name, records):
        (out / name).write_text("".join(json.dumps(r) + "\n" for r in records))

    metrics = {
        "consensus_finalized_bytes_sum": 1e6,
        "consensus_finalized_bytes_count": 2.0,
        "consensus_last_decided_view": 4.0,
        "consensus_last_synced_block_height": 2.0,
        "process_cpu_seconds_total": 0.5,
    }
    jsonl(
        "metrics.jsonl",
        (
            {
                "ts": ts,
                "node": node,
                "ok": True,
                "m": {k: v * ts for k, v in metrics.items()},
            }
            for ts in range(90, 175, 5)
            for node in TOPOLOGY["nodes"]
        ),
    )
    jsonl(
        "host.jsonl",
        (
            {
                "ts": ts,
                "cpu": [50 * ts, 0, 0, 50 * ts, 0, 0, 0, 0, 0, 0],
                "mem_avail": 1000 + ts,
                "procs": {"node0": {"cpu_s": 0.5 * ts, "rss": 10 * ts}},
            }
            for ts in range(90, 172, 2)
        ),
    )
    txs: list[dict] = [
        {
            "id": i,
            "node": 0,
            "t_submit": t,
            "t_included": t + 0.8,
            "height": 1000 + i,
            "status": "included",
        }
        for i, t in enumerate(range(110, 151))
    ]
    txs.append(
        {
            "id": 99,
            "node": 0,
            "t_submit": 120.5,
            "t_included": None,
            "height": None,
            "status": "timeout",
        }
    )
    jsonl("load.jsonl", txs)
    heights = [
        {
            "height": tx["height"],
            "validator": tx["t_submit"] + 0.5,
            "query": tx["t_included"],
            "scanned": tx["t_included"] + 1.1,
        }
        for tx in txs[:-1]
    ]
    heights.append(
        {"height": 2000, "validator": 130.0, "query": 130.5, "scanned": None}
    )
    jsonl("heights.jsonl", heights)
    # 1 MB/s until 130 s, then 0.2 MB/s: half of the 0.4 MB/s that the second step's measured
    # half submits (6 transactions in 15 s).
    counters = [
        {"ts": float(ts), "decided_bytes": 1e6 * min(ts, 104 + ts / 5), "timeouts": 0}
        for ts in range(100, 161)
    ]
    jsonl("consensus.jsonl", counters)
    steps: list[netbench.StepWindow] = [
        {
            "rate_mb_s": 1.0,
            "refine": False,
            "t_start": 100.0,
            "t_mid": 115.0,
            "t_end": 130.0,
        },
        {
            "rate_mb_s": 2.0,
            "refine": False,
            "t_start": 130.0,
            "t_mid": 145.0,
            "t_end": 160.0,
        },
    ]
    calib = {"sha256_1t_mb_s": 2000.0, "sha256_mt_mb_s": 8000.0, "fsync_per_s": 300.0}
    files = {
        "load-meta.json": {
            "submit_errors": 0,
            "max_in_flight": 4,
            "cap_waits": 0,
            "missing_payloads": [],
            "drain_s": None,
            "refine_skipped": False,
        },
        "calibration.json": {"before": calib, "after": calib},
        "steps.json": [
            netbench.judge_step(
                s, netbench.BenchConfig(), txs, heights, counters, s["t_end"]
            )
            for s in steps
        ],
        "sysinfo.json": {"runner": make_result()["runner"]},
        "stake-table.json": {
            "stake_table": [{"stake_table_entry": {"stake_amount": "0x1"}}] * 3
        },
        "run.json": {
            "t0": t0,
            "t1": t1,
            "started": 0.0,
            "wall_s": 300.0,
            "ready_s": 40.0,
            "teardown": [],
            "config_hash": "abc",
            "meta": make_result()["run"],
        },
    }
    for name, data in files.items():
        (out / name).write_text(json.dumps(data))


SCRIPT = Path(__file__).with_name("aws-bench")
REPO = SCRIPT.resolve().parents[2]
_spec = importlib.util.spec_from_loader(
    "aws_bench", SourceFileLoader("aws_bench", str(SCRIPT))
)
assert _spec is not None
awsb = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(awsb)


def parse_plan_args(argv: list[str]) -> "awsb.argparse.Namespace":
    full = ["plan", *argv]
    args = awsb.parse_args(full)
    args.argv = full
    return args


def sts_response(account: str) -> subprocess.CompletedProcess:
    return completed(stdout=json.dumps({"Account": account}))


STS_CALL = ("aws", "--profile", "timeboost-dev", "sts", "get-caller-identity")


def shot_estimate(hosts: list, cfg: "awsb.RunConfig") -> "awsb.Estimate":
    return awsb.cost_estimate(hosts, cfg, None, *awsb.shot_seconds(cfg))


def isolated_env(test: unittest.TestCase, name: str = "run1") -> Path:
    """A fixed fleet name. Returns the out root under the cwd, which the `isolated` fixture
    makes a temp dir."""
    cwd = Path.cwd().resolve()
    if cwd in (REPO, REPO / "scripts"):
        raise AssertionError(f"{test.id()} writes OUT_ROOT into {cwd}: use `isolated`")
    patch = unittest.mock.patch.object(awsb, "default_run_name", return_value=name)
    patch.start()
    test.addCleanup(patch.stop)
    return cwd / awsb.OUT_ROOT


def temp_dir(test: unittest.TestCase) -> Path:
    tmp = Path(tempfile.mkdtemp())
    test.addCleanup(shutil.rmtree, tmp)
    return tmp


def plan_args(*extra: str, nodes: str = "2") -> "awsb.argparse.Namespace":
    return parse_plan_args(["--tag", "x", "--nodes", nodes, *extra])


def fleet(n: int) -> dict:
    hosts = {"ctl": host_info("ctl", "ctl", 1)}
    for i in range(n):
        role = "query" if i == 0 else "validator"
        hosts[f"node{i}"] = host_info(f"node{i}", role, i + 2)
    return hosts


DOTENV_TEXT = f"""# fixture in the syntax of the repo's .env
ESPRESSO_ETH_MNEMONIC="{awsb.BENCH_MNEMONIC}"
ESPRESSO_ORCHESTRATOR_PORT={awsb.ORCHESTRATOR_PORT}
ESPRESSO_L1_PORT={awsb.L1_PORT}
ESPRESSO_STATE_RELAY_SERVER_PORT={awsb.RELAY_PORT}
ESPRESSO_FEE_CONTRACT_PROXY_ADDRESS=0x0000000000000000000000000000000000000001
ESP_TOKEN_PROXY_ADDRESS=0x0000000000000000000000000000000000000002
ESPRESSO_STAKE_TABLE_PROXY_ADDRESS=0x0000000000000000000000000000000000000003
ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS=0x0000000000000000000000000000000000000004
ESPRESSO_ETH_MULTISIG_ADDRESS=a0Ee7A142d267C1f36714E4a8F75612F20a79720
ESPRESSO_OPS_TIMELOCK_ADMIN=${{ESPRESSO_ETH_MULTISIG_ADDRESS}}
"""


def fake_image(ref: str) -> dict:
    return {
        "ref": ref,
        "digest": f"sha256:{'0' * 64}",
        "revision": "abc1234",
        "platforms": [],
    }


def fake_images() -> dict:
    return {
        name: fake_image(f"ghcr.io/x/{name}:t") for name in awsb.IMAGE_COMPONENTS
    } | {name: fake_image(ref) for name, ref in awsb.SUPPORT_IMAGES.items()}


FakeRunner.ready_digests = {
    name: f"{image['ref']}@{image['digest']}" for name, image in fake_images().items()
}


def fake_preflight() -> dict:
    return {
        "account": "027574771971",
        "az": "eu-west-1b",
        "ami_id": "ami-0abc",
        "images": fake_images(),
        "git_diff": None,
    }


DONE_STATE = {
    "phase": "done",
    "detail": "load finished",
    "ready_s": 30.0,
    "t0": FAKE_EPOCH + 100.0,
    "t1": FAKE_EPOCH + 200.0,
}


DESCRIBE = json.dumps(
    [
        {
            "launch": "2026-09-29T15:00:00+00:00",
            "reason": "User initiated (2026-09-29 15:30:00 GMT)",
        }
    ]
)


def valid_result(valid: bool = True) -> dict:
    limit = {"mb_s": 8.0, "bounded": False}
    return {
        "validity": {"valid": valid, "noisy": False, "reasons": []},
        "run": {"sha": "a" * 40, "pr": None, "event": "aws", "ref": "x"},
        "config_hash": "h",
        "capacity": {
            "overall": limit,
            "consensus": limit,
            "query_node": limit,
            "failed_at_mb_s": None,
            "fail_rule": None,
        },
        "steps": [
            {"passed": True, "decided_mb_s": 8.0, "query_lag_ms": {"p99": 120.0}}
        ],
    }


class RunHarness:
    """A temp working dir with its own out root for driving `cmd_run` end to end."""

    def __init__(
        self, test: unittest.TestCase, name: str = "run1", confirmed: bool = False
    ):
        self.yes = not confirmed
        self.answer = confirmed
        self.tmp = Path.cwd()
        self.out = isolated_env(test, name)
        self.name = name
        self.fleet_dir = awsb.OUT_ROOT / name
        self.run_dir = self.fleet_dir / "runs" / "01-run"
        patches = [
            unittest.mock.patch.object(
                awsb, "preflight", return_value=fake_preflight()
            ),
            unittest.mock.patch.object(awsb, "DOTENV", awsb.parse_dotenv(DOTENV_TEXT)),
        ]
        for patch in patches:
            patch.start()
            test.addCleanup(patch.stop)

    def args(self, *extra: str) -> "awsb.argparse.Namespace":
        argv = [
            "run",
            "--tag",
            "t",
            "--nodes",
            "2",
            *(["--yes"] if self.yes else []),
            *extra,
        ]
        args = awsb.parse_args(argv)
        args.argv = argv
        return args

    def run(self, runner: FakeRunner, extra=()) -> int:
        return awsb.cmd_run(
            self.args(*extra), FakeSystem(run=runner, answer=self.answer)
        )

    def index_log(self) -> str:
        return (self.fleet_dir / "driver.log").read_text()

    def last_log_line(self) -> str:
        return (self.fleet_dir / "driver.log").read_text().splitlines()[-1]

    def index(self) -> str:
        return (self.out / "INDEX.md").read_text()


class Scripted:
    """Runner answering ssh by substring of the remote command, and rsync with `rsync_rc`.
    `answers` maps a substring to a list of results; the last result repeats."""

    def __init__(self, answers, rsync_rc: int = 0):
        self.answers = {k: list(v) for k, v in answers.items()}
        self.rsync_rc = rsync_rc
        self.calls: list[list[str]] = []
        self.lock = threading.Lock()

    def __call__(self, argv, env=None):
        with self.lock:
            self.calls.append(argv)
            if argv[0] == "rsync":
                return completed(returncode=self.rsync_rc, stderr="rsync broke")
            for needle, results in self.answers.items():
                if needle in argv[-1]:
                    return results.pop(0) if len(results) > 1 else results[0]
        return completed()


def tmp_dir(test: unittest.TestCase) -> Path:
    tmp = Path(tempfile.mkdtemp())
    test.addCleanup(shutil.rmtree, tmp)
    return tmp


def scripted_remote(
    test: unittest.TestCase, runner, tmp: Path | None = None
) -> "awsb.Remote":
    tmp = tmp or tmp_dir(test)
    return awsb.Remote(runner, tmp, Path("~/.ssh/id"), two_node_hosts_info())


def pg_settings(**overrides) -> dict:
    """pg-settings.json of a node0 whose container took every PG_TUNING value."""
    settings = {key: setting for key, (_, setting) in awsb.PG_TUNING.items()}
    return {**settings, **overrides}


def aws_manifest(steal_hosts: bool = True) -> dict:
    cfg = awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    return {
        "name": "run1",
        "fleet": "run1",
        "query_db": "colocated",
        "hosts": awsb.plan_hosts(cfg),
        "images": fake_images(),
        "start_spread_s": 0.5,
    }


def host_sample(util: float = 0.3, steal: float = 0.0) -> dict:
    return {
        "util_mean": util,
        "util_max": util,
        "steal_pct": steal,
        "iowait_pct": 0.0,
        "mem_avail_min_bytes": 1,
        "disk_mb_s": 1.0,
        "net_mb_s": 1.0,
    }


def clean_evidence() -> dict:
    return {
        "coverage": {"ctl": 1.0, "node0": 1.0, "node1": 1.0},
        "journal_bytes": {"node0": 1_000_000, "node1": 1_000_000},
        "clock_offset_ms": {"ctl": 0.2, "node0": 0.3, "node1": 0.4},
        "digest_mismatch": {"ctl": [], "node0": [], "node1": []},
        "ebs_balance_min": {"EBSByteBalance%": 100.0, "EBSIOBalance%": 100.0},
    }


def clean_result() -> dict:
    return {
        "validity": {"valid": True, "noisy": False, "reasons": []},
        "hosts": {n: host_sample() for n in ("ctl", "node0", "node1")},
    }


def balance_file(byte: list[float], io: list[float], first: float = 60.0) -> dict:
    """`cloudwatch/ec2-node0.json` with one period per item from `first`."""
    times = [first + 60.0 * i for i in range(len(byte))]
    return {
        "instance_id": "i-1",
        "period_s": 60,
        "start": 40.0,
        "end": 220.0,
        "metrics": {
            "EBSByteBalance%": {"timestamps": times, "values": byte},
            "EBSIOBalance%": {"timestamps": times, "values": io},
        },
    }


def write_collected_run(run_dir: Path) -> dict:
    """A 3-node run dir as `cmd_run` leaves it after collection: netbench's synthetic run plus
    manifest, config, topology and `hosts/<name>/` files."""
    write_run_dir(run_dir)
    cfg = awsb.RunConfig(tag="x", nodes=3, load=netbench.BenchConfig(submit_nodes=2))
    hosts = awsb.plan_hosts(cfg)
    manifest = {
        "name": "run1",
        "fleet": "run1",
        "query_db": "colocated",
        "config": awsb.config_to_json(cfg),
        "hosts": hosts,
        "images": fake_images(),
        "az": "eu-west-1b",
        "ami_id": "ami-0abc",
        "start_spread_s": 0.3,
        "cost_usd": {"expected": 1.0, "bound": 2.0},
    }
    netbench.write_json(run_dir / "manifest.json", manifest)
    netbench.write_json(run_dir / "config.json", dataclasses.asdict(cfg.load))
    netbench.write_json(run_dir / "topology.json", TOPOLOGY)
    tracking = "System time     : 0.000010000 seconds fast of NTP time\n"
    for host in hosts:
        host_dir = run_dir / "hosts" / host["name"]
        host_dir.mkdir(parents=True)
        samples = (
            {
                "ts": float(ts),
                "cpu": [50 * ts, 0, 0, 50 * ts, 0, 0, 0, 0, 0, 0],
                "mem_avail": 1000,
                "procs": {},
                "disk": {"nvme0n1": {"read_bytes": 0, "write_bytes": ts * 1_000_000}},
                "net": {"ens5": {"rx_bytes": ts * 500_000, "tx_bytes": ts * 500_000}},
            }
            for ts in range(90, 172, 2)
        )
        netbench.write_jsonl(host_dir / "host.jsonl", samples)
        (host_dir / "du-journal.txt").write_text("1000000\t/data/journal\n")
        (host_dir / "chrony.txt").write_text(tracking)
        digests = {n: f"{i['ref']}@{i['digest']}" for n, i in fake_images().items()}
        netbench.write_json(
            host_dir / "ready.json",
            {"digests": digests, "chronyc_tracking": tracking},
        )
    (run_dir / "cloudwatch").mkdir()
    (run_dir / awsb.EC2_NODE0_FILE).write_text(
        json.dumps(balance_file([100.0, 100.0, 100.0], [100.0, 100.0, 100.0]))
    )
    node0 = run_dir / "hosts" / "node0"
    (node0 / "lscpu.txt").write_text("CPU(s): 16\nModel name: Neoverse-V2\nFlags: fp\n")
    (node0 / "meminfo.txt").write_text("MemTotal:       32000000 kB\n")
    (node0 / "uname.txt").write_text("6.8.0-aws\n")
    return manifest


NOW = datetime.fromtimestamp(FAKE_EPOCH, UTC)


EXPIRES_LATER = "2026-09-29T17:00:00Z"


EXPIRES_PAST = "2026-09-29T15:00:00Z"


def arn(kind: str, resource_id: str) -> str:
    return f"arn:aws:ec2:eu-west-1:1:{kind}/{resource_id}"


def tag_mapping(
    kind: str, resource_id: str, run: str, owner: str | None, expires: str | None
) -> dict:
    tags = {awsb.TAG_RUN: run}
    if owner:
        tags[awsb.TAG_OWNER] = owner
    if expires:
        tags[awsb.TAG_EXPIRES] = expires
    return {
        "arn": arn(kind, resource_id),
        "tags": [{"Key": k, "Value": v} for k, v in tags.items()],
    }


def instance(resource_id: str, state: str = "running", launch: str | None = None):
    return {
        "id": resource_id,
        "type": "c8g.4xlarge",
        "state": state,
        "launch": launch or "2026-09-29T15:30:00+00:00",
    }


class TagRunner(FakeRunner):
    """Answers the tag API with the `{arn, tags}` mappings its `--tag-filters` select, or with
    bare ARNs for a query that asks for `ResourceARN` only (`tagged_resources`)."""

    mappings: list[dict]
    # Volume ids the tag API still lists although EC2 no longer has them.
    stale_volumes: frozenset[str] = frozenset()

    def __call__(
        self, argv: list[str], env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess:
        if "describe-volumes" in argv:
            self.calls.append(argv)
            ids = [
                resource_id
                for kind, resource_id in (
                    awsb.arn_parts(m["arn"]) for m in self.mappings
                )
                if kind == "volume" and resource_id not in self.stale_volumes
            ]
            return completed(stdout=json.dumps(ids))
        if "resourcegroupstaggingapi" in argv:
            self.calls.append(argv)
            arns = "ResourceTagMappingList[].ResourceARN" in argv
            _, _, run = argv[argv.index("--tag-filters") + 1].partition(",Values=")
            tag = {"Key": awsb.TAG_RUN, "Value": run}
            found = [m for m in self.mappings if not run or tag in m["tags"]]
            body = [m["arn"] for m in found] if arns else found
            return completed(stdout=json.dumps(body))
        return super().__call__(argv)


def tag_runner(
    mappings: list[dict], instances: list[dict], stale_volumes: Iterable[str] = ()
) -> FakeRunner:
    runner = TagRunner(
        {
            STS_CALL: sts_response("027574771971"),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instances",
            ): completed(stdout=json.dumps(instances)),
            ("aws",): completed(),
        }
    )
    runner.mappings = mappings
    runner.stale_volumes = frozenset(stale_volumes)
    return runner


def run_cmd(func, argv: list[str], system: FakeSystem) -> tuple[int, str]:
    parsed = awsb.parse_args(argv)
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = func(parsed, system)
    return code, out.getvalue()


STATUS_DESCRIBE = json.dumps(
    [
        {
            **instance("i-000000000001", launch="2026-09-29T15:00:00+00:00"),
            "reason": "",
        },
        {
            **instance("i-000000000002", launch="2026-09-29T15:00:01+00:00"),
            "reason": "",
        },
        {
            **instance("i-000000000003", launch="2026-09-29T15:00:02+00:00"),
            "reason": "",
        },
    ]
)


def fake_report(run_dir: Path, baseline=None) -> dict:
    # The real report reads these keys from the run manifest a driver command wrote.
    awsb.deployment_meta(
        netbench.read_json(run_dir / "manifest.json"), {"clock_offset_ms": {}}
    )
    result = valid_result()
    netbench.write_json(run_dir / "result.json", result)
    return result


class FleetHarness:
    """A temp working dir with its own out root, and the argv of `up` and `run --fleet` for one
    fleet."""

    def __init__(self, test: unittest.TestCase, name: str = "fleet1"):
        self.tmp = Path.cwd()
        isolated_env(test, name)
        self.name = name
        self.out = awsb.OUT_ROOT
        self.fleet_dir = self.out / name
        patches = [
            unittest.mock.patch.object(
                awsb, "preflight", return_value=fake_preflight()
            ),
            unittest.mock.patch.object(awsb, "write_report", fake_report),
            unittest.mock.patch.object(awsb, "DOTENV", awsb.parse_dotenv(DOTENV_TEXT)),
        ]
        for patch in patches:
            patch.start()
            test.addCleanup(patch.stop)

    def parse(self, *argv: str) -> "awsb.argparse.Namespace":
        args = awsb.parse_args(list(argv))
        args.argv = list(argv)
        return args

    def fleet_flags(self) -> list[str]:
        return [
            "--tag",
            "t",
            "--nodes",
            "2",
            "--yes",
        ]

    def up_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return self.parse("up", *self.fleet_flags(), *extra)

    def single_shot_args(self) -> "awsb.argparse.Namespace":
        return self.parse("run", *self.fleet_flags())

    def run_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return self.parse(
            "run",
            "--fleet",
            str(self.fleet_dir),
            "--yes",
            *extra,
        )

    def up(self, runner, *extra: str) -> int:
        return awsb.cmd_up(self.up_args(*extra), FakeSystem(run=runner))

    def run(self, runner, *extra: str) -> int:
        return self.run_with(FakeSystem(run=runner), *extra)

    def run_with(self, system: FakeSystem, *extra: str) -> int:
        return awsb.cmd_run(self.run_args(*extra), system)

    def fleet(self) -> dict:
        return json.loads((self.fleet_dir / "fleet.json").read_text())

    def set_fleet(self, **changes) -> None:
        netbench.write_json(self.fleet_dir / "fleet.json", {**self.fleet(), **changes})

    def index(self) -> list[str]:
        return (self.out / "INDEX.md").read_text().splitlines()[2:]

    def lock(self) -> Path:
        return self.fleet_dir / awsb.FLEET_LOCK

    def driver_log(self) -> str:
        return (self.fleet_dir / "driver.log").read_text()

    def up_fleet(self, test: unittest.TestCase) -> FakeRunner:
        runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
        test.assertEqual(self.up(runner), awsb.EXIT_OK)
        return runner


def ssh_calls(runner: FakeRunner, since: int = 0) -> list[str]:
    return [" ".join(call) for call in runner.calls[since:] if call[0] == "ssh"]


VOLUME_ID = "vol-0abc123def456"


BY_ID = "/dev/disk/by-id/nvme-Amazon_Elastic_Block_Store_vol0abc123def456"


def volume_runner(
    states: list[dict], volume_id: str | None = VOLUME_ID, **kwargs
) -> FakeRunner:
    """A fleet `FakeRunner` whose `tofu output` has the `pg_volume_id` of a fleet with a Postgres
    volume."""
    runner = FakeRunner(states=states, **kwargs)

    def output(argv: list[str]) -> subprocess.CompletedProcess:
        outputs = json.loads(runner.default(argv).stdout)
        return completed(
            stdout=json.dumps({**outputs, "pg_volume_id": {"value": volume_id}})
        )

    runner.respond("output -json", output)
    return runner


RDS_OUTPUT = {
    "identifier": "espresso-bench-fleet1",
    "endpoint": "espresso-bench-fleet1.abc.eu-west-1.rds.amazonaws.com",
    "port": 5432,
    "engine_version": "18.2",
}


RDS_CREATED = "2026-09-30T11:05:00+00:00"


def db_instance(state: str = "available", applied: str = "in-sync") -> dict:
    return {
        "DBInstanceStatus": state,
        "DBParameterGroups": [
            {"DBParameterGroupName": "g", "ParameterApplyStatus": applied}
        ],
        "InstanceCreateTime": RDS_CREATED,
    }


def rds_metric_data(missing: tuple[str, ...] = (), value: float = 100.0) -> str:
    def result(query_id: str) -> dict:
        points = [] if query_id in missing else [(60.0 * i, value) for i in (1, 2, 3)]
        return {
            "Id": query_id,
            "Label": query_id,
            "Timestamps": [
                datetime.fromtimestamp(ts, UTC).isoformat() for ts, _ in points
            ],
            "Values": [v for _, v in points],
            "StatusCode": "Complete",
        }

    return json.dumps(
        {"MetricDataResults": [result(i) for i in awsb.RDS_METRICS], "Messages": []}
    )


def rds_spec(**overrides) -> dict:
    return {
        "instance_class": "db.m8g.4xlarge",
        "engine_version": "18.2",
        "gb": 400,
        "iops": 12000,
        "mbps": 500,
        **overrides,
    }


def mode(path: Path) -> int:
    return stat.S_IMODE(path.stat().st_mode)


class RdsRunner(FakeRunner):
    """fleet `FakeRunner` with the rds output of `tofu`, and the `aws rds`
    and CloudWatch calls an rds fleet makes. `instances` are the successive
    describe-db-instances answers; the last one repeats."""

    def __init__(
        self,
        states: list[dict],
        instances: list[dict] | None = None,
        events: list[dict] | None = None,
        logs: list[dict] | None = None,
        **kwargs,
    ):
        super().__init__(states=states, **kwargs)
        self.instances = instances or [db_instance()]
        self.events = events or []
        self.logs = logs or []
        self.metrics = rds_metric_data()

    def tofu(self, verb: str) -> subprocess.CompletedProcess:
        if verb == "output":
            output = {
                "hosts": {"value": two_node_hosts_info()},
                "rds": {"value": RDS_OUTPUT},
            }
            return completed(stdout=json.dumps(output))
        return super().tofu(verb)

    def aws(self, argv: list[str]) -> subprocess.CompletedProcess:
        if "describe-db-instances" in argv:
            with self.lock:
                if len(self.instances) > 1:
                    instance = self.instances.pop(0)
                else:
                    instance = self.instances[0]
            return completed(stdout=json.dumps({"DBInstances": [instance]}))
        if "describe-events" in argv:
            return completed(stdout=json.dumps({"Events": self.events}))
        if "get-metric-data" in argv and "AWS/RDS" in " ".join(argv):
            return completed(stdout=self.metrics)
        if "describe-db-log-files" in argv:
            return completed(stdout=json.dumps({"DescribeDBLogFiles": self.logs}))
        if "download-db-log-file-portion" in argv:
            name = argv[argv.index("--log-file-name") + 1]
            return completed(stdout=json.dumps({"LogFileData": f"log of {name}\n"}))
        if "reboot-db-instance" in argv:
            return completed()
        return super().aws(argv)


class RdsHarness(FleetHarness):
    """`FleetHarness` for an rds fleet: preflight resolved the engine minor 18.2."""

    def __init__(self, test: unittest.TestCase, modes: str = "rds"):
        super().__init__(test)
        self.modes = modes
        patch = unittest.mock.patch.object(
            awsb,
            "preflight",
            return_value={**fake_preflight(), "rds_engine_version": "18.2"},
        )
        patch.start()
        test.addCleanup(patch.stop)

    def up_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return super().up_args("--db-modes", self.modes, *extra)

    def up_rds(self, test: unittest.TestCase, **kwargs) -> RdsRunner:
        runner = RdsRunner([DONE_STATE], describe=DESCRIBE, **kwargs)
        test.assertEqual(self.up(runner), awsb.EXIT_OK)
        return runner

    def tfvars(self) -> dict:
        path = self.fleet_dir / "terraform" / "terraform.tfvars.json"
        return json.loads(path.read_text())

    def run_dir(self, name: str) -> Path:
        return self.fleet_dir / "runs" / name


LIST_ROLES = ("aws", "--profile", "timeboost-dev", "iam", "list-roles")


ROLE_ARN = "arn:aws:iam::1:role/espresso-bench/espresso-bench-fleet1"


def resource_arn(service: str, resource: str) -> str:
    return f"arn:aws:{service}:eu-west-1:1:{resource}"


def mapping(arn: str, run: str, owner: str | None, expires: str | None) -> dict:
    tags = {awsb.TAG_RUN: run}
    if owner:
        tags[awsb.TAG_OWNER] = owner
    if expires:
        tags[awsb.TAG_EXPIRES] = expires
    return {"arn": arn, "tags": [{"Key": k, "Value": v} for k, v in tags.items()]}


def rds_fleet_mappings(
    run: str, owner: str, expires: str, instance_id: str = "i-1"
) -> list[dict]:
    name = f"espresso-bench-{run}"
    arns = [
        resource_arn("ec2", f"instance/{instance_id}"),
        resource_arn("ec2", "security-group/sg-1"),
        resource_arn("ec2", "key-pair/key-1"),
        resource_arn("ec2", "volume/vol-1"),
        resource_arn("rds", f"db:{name}"),
        resource_arn("rds", f"subgrp:{name}"),
        resource_arn("rds", f"pg:{name}"),
        resource_arn("scheduler", f"schedule-group/{name}"),
    ]
    return [mapping(arn, run, owner, expires) for arn in arns]


def instance_row(instance_id: str) -> dict:
    return {
        "id": instance_id,
        "type": "c8g.4xlarge",
        "state": "running",
        "launch": "2026-09-29T15:30:00+00:00",
        "reason": "",
    }


def rds_tag_runner(
    mappings: list[dict],
    instances: list[dict],
    roles: list[str] | None = None,
    rds_status: str = "available",
    role_missing: bool = False,
) -> FakeRunner:
    """The tag API, describe-instances, the rds and scheduler calls of a sweep, and the role
    listing: `roles` are the ARNs under the scheduler path."""
    runner = tag_runner(mappings, instances)
    prefix = ("aws", "--profile", "timeboost-dev")
    gone = completed(returncode=254, stderr="NoSuchEntity: gone")
    runner.responses = {
        LIST_ROLES: completed(stdout=json.dumps(roles or [])),
        (*prefix, "iam", "list-role-policies"): (
            gone if role_missing else completed(stdout='["delete-rds"]')
        ),
        (*prefix, "iam", "delete-role-policy"): completed(),
        (*prefix, "iam", "delete-role"): gone if role_missing else completed(),
        (*prefix, "rds", "describe-db-instances"): (
            completed(stdout=json.dumps(rds_status))
            if rds_status
            else completed(returncode=254, stderr="DBInstanceNotFound")
        ),
        **runner.responses,
    }
    return runner


def aws_verbs(runner: FakeRunner) -> list[tuple[str, str]]:
    return [(c[3], c[4]) for c in runner.calls if c[0] == "aws"]
