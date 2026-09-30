"""Host-agnostic core of the network benchmark: the load generator, metrics sampling, analysis,
comparison and markdown rendering. No process management and no host-specific paths; every
network endpoint is passed in as a `Topology`. Imported by the local `bench` driver and, later,
by the AWS driver and its agents.
"""

import argparse
import asyncio
import base64
import dataclasses
import hashlib
import http.client
import itertools
import json
import logging
import math
import os
import random
import re
import statistics
import threading
import time
from collections.abc import Awaitable, Callable, Iterator, Mapping, Sequence
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Literal, NotRequired, Protocol, TypedDict, TypeVar
from urllib.parse import urlsplit

log = logging.getLogger("netbench")
T = TypeVar("T")

# Bump when a metric's meaning changes: config_hash covers the inputs, not the analysis code.
SCHEMA_VERSION = 3
METRIC_PREFIXES = (
    "consensus_",
    "process_",
    "tokio_",
    "hotshot_",
    "sql",
    "storage",
    "internal_",
    "journal_",
)
STEAL_NOISY_PCT = 5.0
DRIFT_NOISY_PCT = 10.0
TRACKER_LAG_NOISY_MS = 1000.0
SCRAPE_OK_MIN = 0.9
MIN_READY_HEIGHT = 5
PROGRESS_S = 30
DRAIN_SLACK_S = 10
CATCHUP_TIMEOUT_S = 600
# Payloads are read a block behind the query node's height: a payload requested as soon as its
# header is stored can start a peer fetch that races the node's own insert.
PAYLOAD_LAG_BLOCKS = 1
MISSING_PAYLOAD_S = 10
# One slow or failed answer from the query node must not end the run.
READ_DEADLINE_S = 30.0
READ_RETRY_S = 1.0
# The payload scan's requests are sequential.
TRACKER_THREADS = 1
HEIGHT_POLL_S = 0.1
COUNTER_POLL_S = 1.0
METRICS_EVERY_S = 1.0
KEEP_UP_RATIO = 0.95
# Step rules, see step_fails.
QUERY_LAG_GROWTH_MS_S = 50.0
BASELINE_RUNS = 10
# GitHub's ubuntu-latest spans CPU models; a baseline run counts only on a like runner.
CALIBRATION_MATCH_PCT = 10.0


class Topology(TypedDict):
    """Node name -> base URL, its role for reporting, and which node runs the query API.
    `roles` has the same keys as `nodes`; `query_node` is one of those keys."""

    nodes: dict[str, str]
    roles: dict[str, str]
    query_node: str


class LoadOutcome(TypedDict):
    """Returned by `drive_load`: when the load steps ran. Call `wait_ready` first."""

    t0: float
    t1: float


class Calibration(TypedDict):
    sha256_1t_mb_s: float
    sha256_mt_mb_s: float
    fsync_per_s: float


class RunnerInfo(TypedDict):
    cpu_model: str
    nproc: int
    affinity: int
    cgroup_cpu_max: str | None
    mhz: list[float]
    flags: list[str]
    mem_total_bytes: int
    kernel: str
    image_os: str | None
    image_version: str | None
    runner_name: str | None


class RunMeta(TypedDict):
    sha: str
    ref: str
    event: str
    run_id: str | None
    run_url: str | None
    pr: int | None
    started_at: str
    wall_s: float
    ready_s: float
    local: bool
    teardown: list[str]


class Window(TypedDict):
    t0: float
    t1: float
    height_start: int
    height_end: int


class Quantiles(TypedDict):
    n: int
    mean: float
    p50: float
    p95: float
    p99: float
    max: float


class OpStats(TypedDict):
    per_s: float
    mean_ms: float
    p50_ms: float | None
    p99_ms: float | None
    busy_frac: float


class NodeStats(TypedDict):
    role: str
    decided_height_end: int
    decided_blocks: int
    view_lag_end: int
    cpu_cores: float
    rss_peak_bytes: int
    tokio_busy_frac: float | None
    ops: dict[str, OpStats]


class ProcStats(TypedDict):
    cpu_cores_mean: float
    cpu_s: float
    rss_peak_bytes: int


class HostStats(TypedDict):
    util_mean: float
    util_max: float
    steal_pct: float
    iowait_pct: float
    mem_avail_min_bytes: int


class HostSample(HostStats):
    """Adds throughput sampled by the AWS host monitor; absent for the local bench."""

    disk_mb_s: float
    net_mb_s: float


class LoadStats(TypedDict):
    """Over the whole load, warmup included."""

    submitted: int
    included: int
    timeouts: int
    submit_errors: int
    max_in_flight: int
    cap_waits: int
    missing_payloads: list[int]
    # Per height: the bench finished scanning its payload after it could start.
    tracker_lag_ms: Quantiles | None
    # Before the refine step; skipped if the failed step's backlog did not drain in time.
    # With keep_going: after the last step, None past CATCHUP_TIMEOUT_S.
    drain_s: float | None
    refine_skipped: bool


class StepWindow(TypedDict):
    """One load step: `rate_mb_s` from `t_start`, measured from `t_mid` to `t_end`."""

    rate_mb_s: float
    refine: bool
    t_start: float
    t_mid: float
    t_end: float


class StepMeasures(TypedDict):
    # Theil-Sen slope of the decided payload bytes.
    decided_mb_s: float | None
    timeouts: int | None
    consensus_latency_ms: Quantiles | None
    query_lag_ms: Quantiles | None
    query_lag_slope_ms_s: float | None


class StepResult(StepWindow, StepMeasures):
    # Submit to the block showing on the query node's API: what its client sees.
    latency_ms: Quantiles | None
    mean_view_ms: float | None
    cpu_s_per_mb: float | None
    node_cpu: dict[str, float]
    postgres_cpu: float | None
    consensus_fails: list[str]
    query_fails: list[str]
    passed: bool


class Limit(TypedDict):
    # None: below the first step. Not `bounded`: every step passed, a lower bound.
    mb_s: float | None
    bounded: bool


class Capacity(TypedDict):
    overall: Limit
    consensus: Limit
    query_node: Limit
    failed_at_mb_s: float | None
    fail_rule: str | None


class Validity(TypedDict):
    valid: bool
    noisy: bool
    reasons: list[str]


class CalibrationPair(TypedDict):
    before: Calibration
    after: Calibration
    drift_pct: float


DbMode = Literal["colocated", "volume", "rds"]


class QueryDbMeta(TypedDict):
    """Where the query node's Postgres ran. `store` is the backing volume, `{type, gb, iops,
    mbps}` plus `volume_id` (ebs) or `identifier` (rds-gp3); `tuning` is the applied settings."""

    mode: DbMode
    engine: str
    store: dict[str, Any]
    tls: bool
    tuning: dict[str, str]
    instance_class: NotRequired[str]


class DeploymentMeta(TypedDict):
    """AWS fleet metadata; absent for the local bench."""

    provider: Literal["aws"]
    account: str
    region: str
    az: str
    ami: str
    hosts: dict[str, dict[str, Any]]  # name -> {instance_type: str, ...}
    images: dict[str, str]  # name -> "ref@digest"
    image_revision: str | None
    start_spread_s: float
    clock_offset_ms_max: float
    cost_usd: dict[str, float]  # expected, bound required; actual optional
    fleet: NotRequired[str]
    run_index: NotRequired[int]
    query_db: NotRequired[QueryDbMeta]
    node_env: NotRequired[
        list[str]
    ]  # `KEY=VALUE` overrides of every node's environment


class BenchResult(TypedDict):
    schema_version: int
    run: RunMeta
    runner: RunnerInfo
    calibration: CalibrationPair
    config: dict[str, Any]
    config_hash: str
    window: Window
    steps: list[StepResult]
    capacity: Capacity
    nodes: dict[str, NodeStats]
    processes: dict[str, ProcStats]
    host: HostStats
    load: LoadStats
    stake_table: list[str]
    validity: Validity
    hosts: NotRequired[dict[str, HostSample]]
    deployment: NotRequired[DeploymentMeta]


@dataclass(frozen=True)
class BenchConfig:
    genesis: str = "scripts/network-bench/genesis.toml"
    tx_size: int = 1_000_000
    namespaces: tuple[int, int] = (10000, 10001)
    # MB/s of each load step; each is held `step_s`, and its second half is measured.
    steps: tuple[float, ...] = (4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0)
    step_s: int = 30
    submit_nodes: int = 3
    workers: int = 6
    # Safety cap on transactions in flight: this many seconds of the current step's load.
    cap_s: float = 5.0
    tx_timeout_s: int = 30
    # At the first step's rate.
    warmup_s: int = 60
    seed: int = 42
    # A step fails above either target, see step_fails.
    latency_target_ms: int = 1000
    query_lag_target_ms: int = 1000
    # Every step runs whatever its verdict, no refine step; then the backlog drains, see
    # run_staircase. In flight and tx timeouts stay bound by `cap_s` and `tx_timeout_s`.
    keep_going: bool = False


def parse_rates(text: str) -> tuple[float, ...]:
    """`4,6,8`: increasing MB/s."""
    rates = tuple(float(x) for x in text.split(","))
    if list(rates) != sorted(set(rates)) or rates[0] <= 0:
        raise argparse.ArgumentTypeError(
            f"steps must be increasing and positive: {text}"
        )
    return rates


class Row(TypedDict):
    key: str
    label: str
    unit: str
    better: str
    current: float | None
    baseline: float | None
    delta_pct: float | None
    threshold_pct: float
    verdict: str


class Baseline(TypedDict):
    runs: list[BenchResult]
    error: str | None
    # `main`: stats-fetch output of main runs; `reference`: one result.json given locally.
    source: Literal["main", "reference"]


class Comparison(TypedDict):
    n: int
    excluded: dict[str, int]
    error: str | None
    source: Literal["main", "reference"]
    used: list[RunMeta]
    capacity: list["CapacityRow"]
    steps: list["StepComparison"]


class CapacityRow(TypedDict):
    label: str
    current: Limit
    # The baseline runs' median, a run below the first step ranking lowest.
    baseline: Limit | None
    n: int
    delta_mb_s: float | None
    resolution_mb_s: float
    verdict: str


class StepComparison(TypedDict):
    rate_mb_s: float
    # Baseline runs that ran this rate.
    n: int
    rows: list[Row]


class Clock(Protocol):
    """Every wait and every elapsed-time read of the load path, so tests can run it on virtual
    time."""

    def time(self) -> float: ...

    def monotonic(self) -> float: ...

    def sleep(self, s: float) -> None: ...

    def wait(self, event: threading.Event, s: float) -> bool: ...

    async def asleep(self, s: float) -> None: ...

    async def wait_for(self, aw: Awaitable[T], s: float) -> T: ...


class SystemClock:
    def time(self) -> float:
        return time.time()

    def monotonic(self) -> float:
        return time.monotonic()

    def sleep(self, s: float) -> None:
        time.sleep(s)

    def wait(self, event: threading.Event, s: float) -> bool:
        return event.wait(s)

    async def asleep(self, s: float) -> None:
        await asyncio.sleep(s)

    async def wait_for(self, aw: Awaitable[T], s: float) -> T:
        return await asyncio.wait_for(aw, s)


SYSTEM_CLOCK = SystemClock()


class Http(Protocol):
    """What the load path needs of an HTTP client. `clock` times its retrying readers,
    `closed` tells them to give up."""

    clock: Clock
    closed: threading.Event

    def request(
        self, method: str, url: str, body: bytes | None = None, timeout: float = 10.0
    ) -> tuple[int, bytes]: ...

    def close(self) -> None: ...


# Builds one client per thread pool: `HttpPool` or a test double's `connect`.
HttpFactory = Callable[[Clock], Http]


class HttpPool:
    """Keep-alive connections shared by threads, one request per connection at a time. A
    reused connection the server closed is retried once on a fresh one; any other failure
    raises OSError. `closed` tells retrying readers to give up."""

    def __init__(self, clock: Clock = SYSTEM_CLOCK) -> None:
        self.clock = clock
        self._idle: dict[str, list[http.client.HTTPConnection]] = {}
        self._lock = threading.Lock()
        self.closed = threading.Event()

    def request(
        self, method: str, url: str, body: bytes | None = None, timeout: float = 10.0
    ) -> tuple[int, bytes]:
        parts = urlsplit(url)
        headers = {"Accept": "application/json"}
        if body is not None:
            headers["Content-Type"] = "application/json"
        for attempt in range(2):
            with self._lock:
                idle = self._idle.setdefault(parts.netloc, [])
                conn = idle.pop() if idle else None
            reused = conn is not None
            if conn is None:
                conn = http.client.HTTPConnection(parts.netloc, timeout=timeout)
            elif conn.sock is not None:
                conn.sock.settimeout(timeout)
            try:
                conn.request(method, parts.path, body=body, headers=headers)
                resp = conn.getresponse()
                data = resp.read()
            except (OSError, http.client.HTTPException) as err:
                conn.close()
                if not reused or attempt:
                    raise OSError(f"{method} {url}: {err}") from err
                continue
            with self._lock:
                idle.append(conn)
            return resp.status, data
        raise AssertionError("unreachable")

    def close(self) -> None:
        self.closed.set()
        with self._lock:
            for conns in self._idle.values():
                for conn in conns:
                    conn.close()
            self._idle.clear()


class NetworkError(Exception):
    pass


def drive_load(
    cfg: BenchConfig,
    topo: Topology,
    out: Path,
    alive: Callable[[], bool],
    clock: Clock = SYSTEM_CLOCK,
    http: HttpFactory = HttpPool,
) -> LoadOutcome:
    """Saves the stake table, runs the load staircase, then writes every node's final metrics
    snapshot. The caller must have already waited for readiness with `wait_ready`. Raises
    NetworkError if the network died since or the load generator hits an unrecoverable error."""
    if not alive():
        raise NetworkError("network process exited before load")
    pool = http(clock)
    try:
        query_url = topo["nodes"][topo["query_node"]]
        stake_table = get_ok(pool, query_url + "/v1/node/stake-table/current")
        (out / "stake-table.json").write_bytes(stake_table)
        log.info(
            "load: warmup %d s, then steps of %d s at %s MB/s",
            cfg.warmup_s,
            cfg.step_s,
            ", ".join(map(fmt_num, cfg.steps)),
        )
        validators = [
            url for node, url in topo["nodes"].items() if node != topo["query_node"]
        ]
        submit_urls = [*validators, query_url]
        t0, t1 = asyncio.run(
            generate_load(cfg, submit_urls, query_url, validators, out, clock, http)
        )
        for node, url in topo["nodes"].items():
            prom = get_ok(pool, url + "/v1/status/metrics")
            (out / f"final-{node}.prom").write_bytes(prom)
    finally:
        pool.close()
    return {"t0": t0, "t1": t1}


def wait_ready(
    pool: Http,
    urls: dict[str, str],
    min_height: int,
    timeout_s: float,
    alive: Callable[[], bool],
    clock: Clock = SYSTEM_CLOCK,
) -> float:
    start = clock.time()
    while True:
        heights = {node: block_height(pool, url) for node, url in urls.items()}
        elapsed = clock.time() - start
        match ready_status(heights, min_height, elapsed, timeout_s, alive()):
            case "ready":
                return elapsed
            case "dead":
                raise NetworkError("network process exited during startup")
            case "timeout":
                raise NetworkError(
                    f"network not ready after {timeout_s:.0f} s: heights {heights}"
                )
        clock.sleep(2)


def ready_status(
    heights: dict[str, int | None],
    min_height: int,
    elapsed: float,
    timeout_s: float,
    alive: bool,
) -> Literal["ready", "wait", "dead", "timeout"]:
    """Verdict of one `wait_ready` poll; a node whose API does not answer has height None."""
    if all(h is not None and h >= min_height for h in heights.values()):
        return "ready"
    if not alive:
        return "dead"
    if elapsed > timeout_s:
        return "timeout"
    return "wait"


def block_height(pool: Http, url: str) -> int | None:
    """None until the node's API answers."""
    try:
        status, body = pool.request("GET", url + "/v1/status/block-height", timeout=2)
    except OSError:
        return None
    return int(body) if status == 200 else None


def get_ok(pool: Http, url: str) -> bytes:
    status, body = read(pool, url)
    if status != 200:
        raise NetworkError(f"GET {url}: HTTP {status}")
    return body


def read(pool: Http, url: str) -> tuple[int, bytes]:
    """GET, retrying failed requests and 5xx answers until READ_DEADLINE_S or until the
    pool is closed."""
    deadline = pool.clock.time() + READ_DEADLINE_S
    for attempt in itertools.count():
        try:
            status, body = pool.request("GET", url)
        except OSError as err:
            problem = str(err)
        else:
            if status < 500:
                return status, body
            problem = f"GET {url}: HTTP {status}"
        if pool.clock.time() >= deadline:
            raise NetworkError(f"{problem}, retried for {READ_DEADLINE_S:.0f} s")
        log.log(logging.DEBUG if attempt else logging.WARNING, "%s, retrying", problem)
        if pool.clock.wait(pool.closed, READ_RETRY_S):
            raise NetworkError(f"{problem}, stopped retrying")
    raise AssertionError("unreachable")


def query_height(pool: Http, query_url: str) -> int:
    status, body = read(pool, query_url + "/v1/node/block-height")
    if status != 200:
        raise NetworkError(f"block height from query node: HTTP {status}")
    return int(body)


def validator_height(pool: Http, url: str) -> int:
    """Blocks decided on a validator: its status API reports the newest decided height."""
    status, body = read(pool, url + "/v1/status/block-height")
    if status != 200:
        raise NetworkError(f"block height from {url}: HTTP {status}")
    return int(body) + 1


def block_payload(pool: Http, query_url: str, height: int) -> bytes | None:
    """Raw payload bytes of block `height`, or None if the query node does not have it yet."""
    url = f"{query_url}/v1/availability/payload/{height}"
    status, body = read(pool, url)
    if status == 404:
        return None
    if status != 200:
        raise NetworkError(f"payload {height} from query node: HTTP {status}")
    return base64.b64decode(json.loads(body)["data"]["raw_payload"])


@dataclass
class Tx:
    id: int
    node: int
    # inf while the request waits for a submit thread, so the timeout clock starts at send.
    t_submit: float = math.inf
    t_included: float | None = None
    height: int | None = None
    status: str = "pending"


@dataclass
class Client:
    """HTTP on its own threads, so requests of one kind never queue behind another's."""

    pool: Http
    executor: ThreadPoolExecutor

    async def call(self, fn: Callable[..., T], *args: Any) -> T:
        loop = asyncio.get_running_loop()
        return await loop.run_in_executor(self.executor, fn, self.pool, *args)

    def close(self) -> None:
        self.executor.shutdown(wait=False, cancel_futures=True)
        self.pool.close()


class LoadState:
    """Transactions of one run. At most `cap` are pending: submitted but neither included nor
    timed out."""

    def __init__(self) -> None:
        self.cap = 1
        self.room = asyncio.Event()
        self.txs: list[Tx] = []
        self.pending: dict[int, Tx] = {}
        self.max_in_flight = 0
        self.cap_waits = 0
        self.submit_errors = 0
        self.missing_payloads: list[int] = []

    def submitted(self, tx: Tx) -> None:
        self.txs.append(tx)
        self.pending[tx.id] = tx
        self.max_in_flight = max(self.max_in_flight, len(self.pending))

    def failed(self, tx: Tx) -> None:
        """A submit that failed, unless the transaction was included anyway, e.g. after a
        timed-out response."""
        self.submit_errors += 1
        if self.pending.pop(tx.id, None) is not None:
            self.txs.remove(tx)
            self.room.set()

    async def wait_for_room(self, until: float, clock: Clock) -> bool:
        """False if there is no room before `until`."""
        if len(self.pending) >= self.cap:
            self.cap_waits += 1
        while len(self.pending) >= self.cap:
            self.room.clear()
            try:
                await clock.wait_for(self.room.wait(), until - clock.time())
            except TimeoutError:
                return False
        return True

    def resolve(self, tx_id: int, status: str, now: float) -> None:
        tx = self.pending.pop(tx_id, None)
        if tx is None:
            return
        tx.status = status
        if status == "included":
            tx.t_included = now
        self.room.set()

    def include(self, tx_id: int, height: int, at: float) -> None:
        tx = self.pending.get(tx_id)
        if tx is not None:
            tx.height = height
            self.resolve(tx_id, "included", at)


async def generate_load(
    cfg: BenchConfig,
    submit_urls: list[str],
    query_url: str,
    validator_urls: list[str],
    out: Path,
    clock: Clock,
    http: HttpFactory,
) -> tuple[float, float]:
    """The load staircase, then up to `tx_timeout_s` for the stragglers. Returns when the
    steps started and ended. Writes one line per transaction to load.jsonl, per block height
    to heights.jsonl, per consensus counter sample of the first validator to
    consensus.jsonl, per step to steps.json, and the counters to load-meta.json."""
    state = LoadState()
    marker = hashlib.sha256(f"network-bench:{cfg.seed}".encode()).digest()[:16]
    bodies = tx_bodies(cfg, len(marker) + 8)
    submitter = Client(http(clock), ThreadPoolExecutor(cfg.workers))
    tracking = Client(http(clock), ThreadPoolExecutor(TRACKER_THREADS))
    polling = Client(http(clock), ThreadPoolExecutor(2 + len(validator_urls)))
    done = asyncio.Event()
    counters: list[dict[str, Any]] = []
    heights: Heights | None = None
    steps: list[dict[str, Any]] = []
    drained: float | None = None
    skipped = False
    try:
        heights = Heights(await tracking.call(query_height, query_url))
        async with asyncio.TaskGroup() as group:
            pollers = [
                group.create_task(
                    poll_heights(
                        polling, query_url, query_height, heights, "query", clock
                    )
                ),
                *(
                    group.create_task(
                        poll_heights(
                            polling, url, validator_height, heights, "validator", clock
                        )
                    )
                    for url in validator_urls
                ),
                group.create_task(
                    poll_counters(polling, validator_urls[0], counters, clock)
                ),
            ]
            tracker = group.create_task(
                track_inclusion(
                    cfg,
                    state,
                    tracking,
                    query_url,
                    marker,
                    heights,
                    counters,
                    done,
                    clock,
                )
            )
            load = Load(
                cfg,
                state,
                submitter,
                submit_urls[: cfg.submit_nodes],
                marker,
                bodies,
                clock,
            )
            drained, skipped = await run_staircase(load, heights, counters, steps)
            done.set()
            await tracker
            for poller in pollers:
                poller.cancel()
    except* (NetworkError, OSError) as group:
        raise NetworkError(f"load generator: {innermost(group)}") from group
    finally:
        submitter.close()
        tracking.close()
        polling.close()
        # In `finally` so a load cut short (SIGTERM of the AWS agent) keeps its raw data.
        if heights is not None:
            write_load_files(
                out, state, heights, counters, steps, marker, drained, skipped
            )
    return steps[0]["t_start"], steps[-1]["t_end"]


def write_load_files(
    out: Path,
    state: LoadState,
    heights: "Heights",
    counters: list[dict[str, Any]],
    steps: list[dict[str, Any]],
    marker: bytes,
    drained: float | None,
    skipped: bool,
) -> None:
    # A load cut short leaves txs still queued for a submit thread (`t_submit` inf, not JSON).
    sent = (tx for tx in state.txs if math.isfinite(tx.t_submit))
    write_jsonl(out / "load.jsonl", (dataclasses.asdict(tx) for tx in sent))
    write_jsonl(out / "heights.jsonl", heights.records())
    write_jsonl(out / "consensus.jsonl", iter(counters))
    if steps:
        write_json(out / "steps.json", steps)
    write_json(
        out / "load-meta.json",
        {
            "start_height": heights.start,
            "max_in_flight": state.max_in_flight,
            "cap_waits": state.cap_waits,
            "submit_errors": state.submit_errors,
            "missing_payloads": state.missing_payloads,
            "drain_s": drained,
            "refine_skipped": skipped,
            "marker": marker.hex(),
        },
    )


def innermost(err: BaseException) -> BaseException:
    """The first leaf of nested exception groups, as raised by task groups."""
    # ruff assumes Python 3.10 here; the script requires 3.11.
    while isinstance(err, BaseExceptionGroup):  # noqa: F821
        err = err.exceptions[0]
    return err


def tx_bodies(cfg: BenchConfig, header_len: int) -> list[bytes]:
    rng = random.Random(cfg.seed)
    return [rng.randbytes(cfg.tx_size - header_len) for _ in range(16)]


def tx_interval_s(tx_size: int, rate_mb_s: float) -> float:
    return tx_size / (rate_mb_s * 1e6)


def step_cap(cfg: BenchConfig, rate_mb_s: float) -> int:
    return max(1, round(cfg.cap_s / tx_interval_s(cfg.tx_size, rate_mb_s)))


def next_due(due: float, interval: float, now: float) -> float:
    """Evenly spaced submit times. A pacer more than an interval late restarts from `now`
    instead of bursting to catch up."""
    return max(due + interval, now)


@dataclass
class Load:
    cfg: BenchConfig
    state: LoadState
    client: Client
    urls: list[str]
    marker: bytes
    bodies: list[bytes]
    clock: Clock
    ids: Iterator[int] = dataclasses.field(default_factory=itertools.count)


async def run_staircase(
    load: Load,
    heights: "Heights",
    counters: list[dict[str, Any]],
    steps: list[dict[str, Any]],
) -> tuple[float | None, bool]:
    """Warmup, then steps up the ramp until one fails, then one refine step once the failed
    step's backlog drained. Each step is judged on what is known at its end; the report keeps
    that verdict. Appends to `steps` as they end, so a run cut short keeps them. Returns the
    drain time and whether the drain timed out, which skips the refine step. With
    `keep_going` every step of the ramp runs, without a refine step, and the drain follows
    the last step."""
    cfg = load.cfg
    passed: list[bool] = []
    drained, skipped = None, False
    async with asyncio.TaskGroup() as submits:
        await pace(load, submits, cfg.steps[0], load.clock.time() + cfg.warmup_s)
        while (rate := next_rate(cfg.steps, passed, cfg.keep_going)) is not None:
            refine = not cfg.keep_going and not all(passed)
            if refine:
                drained = await drain(
                    load.state,
                    counters,
                    heights,
                    cfg.tx_timeout_s + DRAIN_SLACK_S,
                    load.clock,
                )
                if drained is None:
                    log.warning("backlog did not drain, skipping the refine step")
                    skipped = True
                    break
                log.info("backlog drained in %.1f s", drained)
            start = load.clock.time()
            await pace(load, submits, rate, start + cfg.step_s)
            step: StepWindow = {
                "rate_mb_s": rate,
                "refine": refine,
                "t_start": start,
                "t_mid": start + cfg.step_s / 2,
                "t_end": load.clock.time(),
            }
            txs = [dataclasses.asdict(tx) for tx in load.state.txs]
            judged = judge_step(
                step, cfg, txs, list(heights.records()), counters, load.clock.time()
            )
            fails = judged["consensus_fails"] + judged["query_fails"]
            log_step(judged, fails)
            passed.append(not fails)
            steps.append(judged)
        if cfg.keep_going:
            # Not waiting for pending: a transaction of a lost payload stays pending until its
            # timeout, which `keep_going` runs set above the lag.
            drained = await drain(
                load.state,
                counters,
                heights,
                CATCHUP_TIMEOUT_S,
                load.clock,
                wait_pending=False,
            )
            if drained is None:
                log.warning("backlog did not drain in %d s", CATCHUP_TIMEOUT_S)
            else:
                log.info("backlog drained in %.1f s", drained)
    return drained, skipped


async def drain(
    state: LoadState,
    counters: list[dict[str, Any]],
    heights: "Heights",
    timeout_s: float,
    clock: Clock,
    wait_pending: bool = True,
) -> float | None:
    """Submits nothing until no transaction is pending (unless `wait_pending` is off), decided
    bytes stopped growing and the query node caught up with the validators. Seconds that took,
    or None after `timeout_s`."""
    start, target = clock.time(), None
    while clock.time() - start < timeout_s:
        # Once nothing is pending, two equal samples (~1 s apart) mean consensus is idle.
        flat = (
            len(counters) >= 2
            and counters[-1]["decided_bytes"] == counters[-2]["decided_bytes"]
        )
        pending = len(state.pending) if wait_pending else 0
        if target is None and is_drained(pending, flat):
            # Fixed once settled: empty blocks keep the validator height moving.
            target = heights.top("validator")
        if target is not None and heights.top("query") >= target:
            return clock.time() - start
        await clock.asleep(0.1)
    return None


def is_drained(pending: int, flat: bool) -> bool:
    """True when no transaction is pending and decided bytes stopped growing. `drain` checks
    separately that the query node reached the validators' height."""
    return pending == 0 and flat


def log_step(m: dict[str, Any], fails: list[str]) -> None:
    consensus, lag = m["consensus_latency_ms"], m["query_lag_ms"]
    log.info(
        "step %s MB/s: decided %s MB/s, consensus p50 %s ms, query lag p50 %s ms: %s",
        fmt_num(m["rate_mb_s"]),
        fmt_num(m["decided_mb_s"]),
        fmt_num(consensus["p50"] if consensus else None),
        fmt_num(lag["p50"] if lag else None),
        "; ".join(fails) or "pass",
    )


async def pace(
    load: Load, submits: asyncio.TaskGroup, rate_mb_s: float, until: float
) -> None:
    """Submits at `rate_mb_s` until `until`, independent of inclusion, but never more than
    the step's cap in flight."""
    state = load.state
    state.cap = step_cap(load.cfg, rate_mb_s)
    state.room.set()
    interval = tx_interval_s(load.cfg.tx_size, rate_mb_s)
    clock = load.clock
    due = clock.time()
    while True:
        await clock.asleep(max(0.0, due - clock.time()))
        if clock.time() >= until or not await state.wait_for_room(until, clock):
            return
        tx_id = next(load.ids)
        tx = Tx(id=tx_id, node=tx_id % len(load.urls))
        state.submitted(tx)
        submits.create_task(submit_tx(load, tx))
        due = next_due(due, interval, clock.time())


async def submit_tx(load: Load, tx: Tx) -> None:
    """Pending before the request goes out: its block may be scanned before the response
    arrives."""
    lo, hi = load.cfg.namespaces
    body = load.bodies[tx.id % len(load.bodies)]
    payload = load.marker + tx.id.to_bytes(8, "big") + body
    request = json.dumps(
        {
            "namespace": lo + tx.id % (hi - lo + 1),
            "payload": base64.b64encode(payload).decode(),
        }
    ).encode()
    url = load.urls[tx.node] + "/v1/submit/submit"
    if await load.client.call(post_tx, tx, url, request) != 200:
        load.state.failed(tx)


def post_tx(pool: Http, tx: Tx, url: str, body: bytes) -> int:
    """HTTP status, 0 if the request failed. Stamps `t_submit` as the request goes out, not
    when it was queued for a thread."""
    tx.t_submit = pool.clock.time()
    try:
        status, _ = pool.request("POST", url, body)
    except OSError:
        return 0
    return status


HeightSource = Literal["query", "validator"]


class Heights:
    """From `start` on, when each block height first showed on the query node's API and on
    any validator's status API, and when the bench finished scanning its payload."""

    def __init__(self, start: int) -> None:
        self.start = start
        self.seen: dict[HeightSource, dict[int, float]] = {
            "query": {},
            "validator": {},
        }
        self.scanned: dict[int, float] = {}

    def top(self, source: HeightSource) -> int:
        return self.start + len(self.seen[source])

    def saw(self, source: HeightSource, count: int, now: float) -> None:
        for height in range(self.top(source), count):
            self.seen[source][height] = now

    def records(self) -> Iterator[dict[str, Any]]:
        top = max(self.top("query"), self.top("validator"))
        for height in range(self.start, top):
            yield {
                "height": height,
                "validator": self.seen["validator"].get(height),
                "query": self.seen["query"].get(height),
                "scanned": self.scanned.get(height),
            }


async def poll_heights(
    client: Client,
    url: str,
    count: Callable[[Http, str], int],
    heights: Heights,
    source: HeightSource,
    clock: Clock,
) -> None:
    """On its own thread, so block times never wait for payload scans."""
    while True:
        heights.saw(source, await client.call(count, url), clock.time())
        await clock.asleep(HEIGHT_POLL_S)


async def poll_counters(
    client: Client, url: str, counters: list[dict[str, Any]], clock: Clock
) -> None:
    """Decided payload bytes and view timeouts of one validator, every COUNTER_POLL_S."""
    while True:
        m = await client.call(consensus_counters, url)
        counters.append({"ts": clock.time(), **m})
        await clock.asleep(COUNTER_POLL_S)


def consensus_counters(pool: Http, url: str) -> dict[str, float]:
    m = parse_prom(get_ok(pool, url + "/v1/status/metrics").decode())
    # Both appear only once the first block is decided or view timed out.
    return {
        "decided_bytes": m.get("consensus_finalized_bytes_sum", 0.0),
        "timeouts": m.get("consensus_number_of_timeouts", 0.0),
    }


async def track_inclusion(
    cfg: BenchConfig,
    state: LoadState,
    client: Client,
    query_url: str,
    marker: bytes,
    heights: Heights,
    counters: list[dict[str, Any]],
    done: asyncio.Event,
    clock: Clock,
) -> None:
    """Scan every new block's payload for our marker; time out transactions that never show.
    A transaction counts as included when its block's header appeared on the query node. A
    payload the query node lacks is retried after each scan until MISSING_PAYLOAD_S."""
    pattern = re.compile(re.escape(marker) + b"(.{8})", re.DOTALL)
    deadline = None
    report = clock.time() + PROGRESS_S
    height = heights.start
    missing: dict[int, float] = {}
    while True:
        now = clock.time()
        if now >= report:
            log_progress(state, heights, counters, now)
            report = now + PROGRESS_S
        if done.is_set():
            deadline = deadline or now + cfg.tx_timeout_s
            if not state.pending or (now > deadline and not missing):
                for tx_id in list(state.pending):
                    state.resolve(tx_id, "timeout", now)
                return
        for tx in [
            tx for tx in state.pending.values() if now - tx.t_submit > cfg.tx_timeout_s
        ]:
            state.resolve(tx.id, "timeout", now)
        top = heights.top("query")
        while height < top - PAYLOAD_LAG_BLOCKS:
            raw = await client.call(block_payload, query_url, height)
            if raw is None:
                missing[height] = heights.seen["query"][height]
            else:
                record_inclusions(state, pattern, raw, heights, height, clock)
            height += 1
        await retry_missing(state, client, query_url, pattern, heights, missing, clock)
        await clock.asleep(0.1)


async def retry_missing(
    state: LoadState,
    client: Client,
    query_url: str,
    pattern: re.Pattern[bytes],
    heights: Heights,
    missing: dict[int, float],
    clock: Clock,
) -> None:
    """One fetch per missing payload. Past MISSING_PAYLOAD_S after its block appeared a
    payload counts as lost; its transactions, which cannot be told apart, time out."""
    for height, at_height in list(missing.items()):
        raw = await client.call(block_payload, query_url, height)
        if raw is None and clock.time() - at_height < MISSING_PAYLOAD_S:
            continue
        del missing[height]
        if raw is None:
            log.warning("payload %d missing on the query node, skipping", height)
            state.missing_payloads.append(height)
        else:
            record_inclusions(state, pattern, raw, heights, height, clock)


def record_inclusions(
    state: LoadState,
    pattern: re.Pattern[bytes],
    raw: bytes,
    heights: Heights,
    height: int,
    clock: Clock,
) -> None:
    at = heights.seen["query"][height]
    for match in pattern.finditer(raw):
        state.include(int.from_bytes(match.group(1), "big"), height, at)
    heights.scanned[height] = clock.time()


class ProgressStats(TypedDict):
    """`None` where the window or the heights hold no data yet."""

    validator: int | None
    query: int | None
    lag_ms: float | None
    blocks_behind: int
    seconds_behind: float
    block_s: float | None
    block_mb: float | None


def progress_window(
    heights: Heights, counters: Sequence[Mapping[str, Any]], now: float, window_s: float
) -> ProgressStats:
    """Network state over the last `window_s`: last height on each source, query lag, and mean
    block time and size from the blocks the validators showed in the window."""
    validator, query = heights.top("validator"), heights.top("query")
    lag_ms = None
    behind = max(validator - query, 0)
    seconds_behind = 0.0
    if behind:
        seconds_behind = now - heights.seen["validator"][query]
    elif validator > heights.start:
        last = validator - 1
        lag_ms = (heights.seen["query"][last] - heights.seen["validator"][last]) * 1000
    blocks = sum(1 for t in heights.seen["validator"].values() if t >= now - window_s)
    inside = [c for c in counters if c["ts"] >= now - window_s]
    return {
        "validator": validator - 1 if validator > heights.start else None,
        "query": query - 1 if query > heights.start else None,
        "lag_ms": lag_ms,
        "blocks_behind": behind,
        "seconds_behind": seconds_behind,
        "block_s": window_s / blocks if blocks else None,
        "block_mb": (inside[-1]["decided_bytes"] - inside[0]["decided_bytes"])
        / blocks
        / 1e6
        if blocks and len(inside) > 1
        else None,
    }


def log_progress(
    state: LoadState,
    heights: Heights,
    counters: Sequence[Mapping[str, Any]],
    now: float,
) -> None:
    stats = progress_window(heights, counters, now, PROGRESS_S)
    included = sum(1 for tx in state.txs if tx.status == "included")
    timeouts = sum(1 for tx in state.txs if tx.status == "timeout")
    log.info(
        "height v=%s q=%s (%s), block %s s, %s MB; %d submitted, %d included, %d pending, "
        "%d timed out, %d waited for the cap",
        fmt_num(stats["validator"]) or "-",
        fmt_num(stats["query"]) or "-",
        fmt_lag(stats),
        fmt_num(stats["block_s"]) or "-",
        fmt_num(stats["block_mb"]) or "-",
        len(state.txs),
        included,
        len(state.pending),
        timeouts,
        state.cap_waits,
    )


def fmt_lag(stats: ProgressStats) -> str:
    if stats["blocks_behind"]:
        return (
            f"{stats['blocks_behind']} blk / "
            f"{fmt_num(stats['seconds_behind'])} s behind"
        )
    if stats["lag_ms"] is None:
        return "no lag yet"
    return f"lag {fmt_num(stats['lag_ms'])} ms"


def next_rate(
    ramp: Sequence[float], passed: list[bool], keep_going: bool = False
) -> float | None:
    """The rate of the next step after steps with `passed` verdicts: up the ramp until a step
    fails, then one refine step halfway back, then none. With `keep_going` up the whole ramp,
    whatever the verdicts."""
    if keep_going or all(passed):
        return ramp[len(passed)] if len(passed) < len(ramp) else None
    first_fail = passed.index(False)
    if first_fail == 0 or len(passed) > first_fail + 1:
        return None
    return (ramp[first_fail - 1] + ramp[first_fail]) / 2


def judge_step(
    step: StepWindow,
    cfg: BenchConfig,
    txs: list[dict[str, Any]],
    heights: list[dict[str, Any]],
    counters: list[dict[str, Any]],
    now: float,
) -> dict[str, Any]:
    """The one verdict on a step, as the ramp takes it at `now`; stored in steps.json."""
    m = step_measures(step, cfg, txs, heights, counters, now)
    consensus, query = step_fails(m, step["rate_mb_s"], cfg)
    return {**step, **m, "consensus_fails": consensus, "query_fails": query}


def step_measures(
    step: StepWindow,
    cfg: BenchConfig,
    txs: list[dict[str, Any]],
    heights: list[dict[str, Any]],
    counters: list[dict[str, Any]],
    now: float,
) -> StepMeasures:
    """Over the measured half, from what is known at `now`. A transaction not yet on a
    validator, and a height not yet on the query node, count with their time so far once that
    exceeds the target."""
    t0, t1 = step["t_mid"], step["t_end"]
    inside = [c for c in counters if t0 <= c["ts"] <= t1]
    first, last = (inside[0], inside[-1]) if len(inside) >= 2 else (None, None)
    on_validator = {h["height"]: h["validator"] for h in heights}
    consensus = []
    for tx in txs:
        if not t0 <= tx["t_submit"] <= t1:
            continue
        seen = on_validator.get(tx["height"])
        if seen is not None:
            consensus.append((seen - tx["t_submit"]) * 1000)
        elif (now - tx["t_submit"]) * 1000 > cfg.latency_target_ms:
            consensus.append((now - tx["t_submit"]) * 1000)
    lags = []
    for h in heights:
        seen = h["validator"]
        if seen is None or not t0 <= seen <= t1:
            continue
        if h["query"] is not None:
            lags.append((seen, (h["query"] - seen) * 1000))
        elif (now - seen) * 1000 > cfg.query_lag_target_ms:
            lags.append((seen, (now - seen) * 1000))
    # A slope over all samples: two samples see whole blocks, off by up to one block.
    decided = theil_sen([(c["ts"], c["decided_bytes"] / 1e6) for c in inside])
    return {
        "decided_mb_s": decided,
        "timeouts": int(last["timeouts"] - first["timeouts"])
        if first and last
        else None,
        "consensus_latency_ms": quantiles(consensus),
        "query_lag_ms": quantiles([lag for _, lag in lags]),
        "query_lag_slope_ms_s": theil_sen(lags),
    }


def theil_sen(points: list[tuple[float, float]]) -> float | None:
    """Median slope over all point pairs: one outlier does not tilt it."""
    slopes = [
        (y2 - y1) / (x2 - x1)
        for (x1, y1), (x2, y2) in itertools.combinations(sorted(points), 2)
        if x2 > x1
    ]
    return statistics.median(slopes) if slopes else None


def step_fails(
    m: StepMeasures, rate: float, cfg: BenchConfig
) -> tuple[list[str], list[str]]:
    """Failed rules of a step: consensus-side, then query-node-side."""
    consensus, query = [], []
    decided = m["decided_mb_s"]
    if decided is None:
        consensus.append("no consensus metrics")
    elif decided < KEEP_UP_RATIO * rate:
        consensus.append(f"decided {decided / rate:.0%} of offered")
    if m["timeouts"]:
        consensus.append(f"{m['timeouts']} view timeouts")
    latency = m["consensus_latency_ms"]
    if latency and latency["p50"] > cfg.latency_target_ms:
        consensus.append(
            f"consensus latency p50 {latency['p50']:.0f} ms > {cfg.latency_target_ms} ms"
        )
    lag, slope = m["query_lag_ms"], m["query_lag_slope_ms_s"]
    if slope is not None and slope > QUERY_LAG_GROWTH_MS_S:
        query.append(f"query lag grows {slope:.0f} ms/s")
    if lag and lag["p50"] > cfg.query_lag_target_ms:
        query.append(
            f"query lag p50 {lag['p50']:.0f} ms > {cfg.query_lag_target_ms} ms"
        )
    return consensus, query


def capacity(steps: Sequence[Mapping[str, Any]]) -> Capacity:
    """Highest passing rate below the lowest failing one, overall and per rule side.

    A side can fail at one rate and pass at a higher one: noise, or a refine step below the
    first failure that trips the other side. Its limit is still below its lowest failing
    rate, so the two limits need not bracket the steps the ramp passed."""
    ordered = sorted(steps, key=lambda s: s["rate_mb_s"])

    def limit(fails: Callable[[Mapping[str, Any]], list[str]]) -> tuple[Limit, Any]:
        passed = None
        for step in ordered:
            if fails(step):
                return {"mb_s": passed, "bounded": True}, step
            passed = step["rate_mb_s"]
        return {"mb_s": passed, "bounded": False}, None

    overall, failed = limit(lambda s: s["consensus_fails"] + s["query_fails"])
    return {
        "overall": overall,
        "consensus": limit(lambda s: s["consensus_fails"])[0],
        "query_node": limit(lambda s: s["query_fails"])[0],
        "failed_at_mb_s": failed["rate_mb_s"] if failed else None,
        "fail_rule": (
            f"{'; '.join(failed['consensus_fails'] + failed['query_fails'])} at "
            f"{fmt_num(failed['rate_mb_s'])} MB/s"
            if failed
            else None
        ),
    }


def capacity_line(cap: Capacity) -> str:
    overall, consensus = cap["overall"], cap["consensus"]
    if not overall["bounded"]:
        return f"Capacity **>= {fmt_num(overall['mb_s'])} MB/s**: no step failed."
    value = (
        fmt_num(overall["mb_s"])
        if overall["mb_s"] is not None
        else f"< {fmt_num(cap['failed_at_mb_s'])}"
    )
    if consensus["bounded"] and consensus["mb_s"] == overall["mb_s"]:
        who = "consensus limits"
    else:
        bound = "limits at" if consensus["bounded"] else ">="
        who = (
            f"query node limits at {value} MB/s, consensus {bound} "
            f"{fmt_num(consensus['mb_s'])} MB/s"
        )
    return f"Capacity **{value} MB/s**: {who} ({cap['fail_rule']})."


def sample_metrics(
    urls: dict[str, str],
    prefixes: tuple[str, ...],
    out: Path,
    stop: threading.Event,
    clock: Clock = SYSTEM_CLOCK,
) -> None:
    """Every METRICS_EVERY_S, one JSON line per node: {ts, node, ok, m}; `m` keeps the
    families in `prefixes`. Often enough for rates over a step's measured half."""
    pool = HttpPool(clock)
    with open(out, "a", buffering=1) as f:
        while not stop.is_set():
            start = clock.time()
            for node, url in urls.items():
                rec: dict[str, Any] = {"ts": clock.time(), "node": node, "ok": False}
                try:
                    status, body = pool.request(
                        "GET", url + "/v1/status/metrics", timeout=4
                    )
                    if status == 200:
                        rec["ok"] = True
                        rec["m"] = {
                            k: v
                            for k, v in parse_prom(body.decode()).items()
                            if k.startswith(prefixes)
                        }
                except OSError:
                    pass
                f.write(json.dumps(rec) + "\n")
            clock.wait(stop, max(0.0, METRICS_EVERY_S - (clock.time() - start)))
    pool.close()


def usable_cpu_jiffies() -> list[int]:
    """/proc/stat counters summed over the CPUs this process may run on, so utilisation under
    taskset or a cpuset is relative to those CPUs."""
    usable = {f"cpu{n}" for n in os.sched_getaffinity(0)}
    rows = [
        [int(x) for x in fields[1:]]
        for fields in map(str.split, Path("/proc/stat").read_text().splitlines())
        if fields and fields[0] in usable
    ]
    return [sum(column) for column in zip(*rows, strict=True)]


def psi(resource: str) -> float | None:
    """`some avg10` pressure, or None where the kernel has no PSI."""
    try:
        line = Path(f"/proc/pressure/{resource}").read_text().split("\n")[0]
    except FileNotFoundError:
        return None
    return float(line.split()[1].split("=")[1])


def meminfo() -> dict[str, int]:
    out = {}
    for line in Path("/proc/meminfo").read_text().splitlines():
        key, value = line.split(":", 1)
        out[key] = int(value.split()[0]) * 1024
    return out


Sample = tuple[float, dict[str, float]]


def analyze(out: Path, cfg: BenchConfig, topo: Topology) -> BenchResult:
    """`window`, `nodes`, `processes`, `host` and `load` cover all steps; each step's own
    metrics cover its measured half."""
    nodes = list(topo["nodes"])
    run = read_json(out / "run.json")
    t0, t1 = run["t0"], run["t1"]
    series = load_series(out / "metrics.jsonl", nodes)
    host = list(read_jsonl(out / "host.jsonl"))
    txs = list(read_jsonl(out / "load.jsonl"))
    heights = list(read_jsonl(out / "heights.jsonl"))
    calib = read_json(out / "calibration.json")
    before, after = calib["before"], calib["after"]
    steps = [
        step_result(step, txs, series, host) for step in read_json(out / "steps.json")
    ]
    result: BenchResult = {
        "schema_version": SCHEMA_VERSION,
        "run": run_meta(run),
        "runner": read_json(out / "sysinfo.json")["runner"],
        "calibration": {
            "before": before,
            "after": after,
            "drift_pct": pct_change(before["sha256_mt_mb_s"], after["sha256_mt_mb_s"]),
        },
        "config": dataclasses.asdict(cfg),
        "config_hash": run["config_hash"],
        "window": window(series, topo["query_node"], t0, t1),
        "steps": steps,
        "capacity": capacity(steps),
        "nodes": {
            node: node_stats(node, topo["roles"][node], series, t0, t1)
            for node in nodes
        },
        "processes": process_stats(host, t0, t1),
        "host": host_stats(host, t0, t1),
        "load": load_stats(out, txs, heights, t0, t1),
        "stake_table": stake_amounts(out / "stake-table.json"),
        "validity": {"valid": True, "noisy": False, "reasons": []},
    }
    result["validity"] = check_validity(
        result, scrape_coverage(out / "metrics.jsonl", nodes, t0, t1), len(nodes)
    )
    return result


def step_result(
    judged: dict[str, Any],
    txs: list[dict[str, Any]],
    series: dict[str, list[Sample]],
    host: list[dict[str, Any]],
) -> StepResult:
    """The ramp's verdict on a step (see judge_step) plus metrics that decide nothing."""
    t0, t1 = judged["t_mid"], judged["t_end"]
    views = [
        rate(s, t0, t1, "consensus_last_decided_view") for s in series.values() if s
    ]
    view = statistics.median(views) if views else None
    node_cpu = {
        node: rate(s, t0, t1, "process_cpu_seconds_total")
        for node, s in series.items()
        if s
    }
    decided = judged["decided_mb_s"]
    procs = process_stats(host, t0, t1)
    fails = judged["consensus_fails"] + judged["query_fails"]
    return {
        "rate_mb_s": judged["rate_mb_s"],
        "refine": judged["refine"],
        "t_start": judged["t_start"],
        "t_mid": t0,
        "t_end": t1,
        "decided_mb_s": decided,
        "timeouts": judged["timeouts"],
        "consensus_latency_ms": judged["consensus_latency_ms"],
        "query_lag_ms": judged["query_lag_ms"],
        "query_lag_slope_ms_s": judged["query_lag_slope_ms_s"],
        "latency_ms": quantiles(
            [
                (tx["t_included"] - tx["t_submit"]) * 1000
                for tx in txs
                if t0 <= tx["t_submit"] <= t1 and tx["status"] == "included"
            ]
        ),
        "mean_view_ms": 1000 / view if view else None,
        "cpu_s_per_mb": sum(node_cpu.values()) / decided if decided else None,
        "node_cpu": node_cpu,
        "postgres_cpu": (
            procs["postgres"]["cpu_cores_mean"] if "postgres" in procs else None
        ),
        "consensus_fails": judged["consensus_fails"],
        "query_fails": judged["query_fails"],
        "passed": not fails,
    }


def check_validity(
    result: BenchResult, coverage: dict[str, float], n_nodes: int
) -> Validity:
    invalid: list[str] = []
    noisy: list[str] = []
    for node, frac in coverage.items():
        if frac < SCRAPE_OK_MIN:
            invalid.append(f"{node} metrics answered for only {frac:.0%} of the window")
    for node, stats in result["nodes"].items():
        if stats["decided_blocks"] <= 0:
            invalid.append(f"{node} decided no blocks in the window")
    for step in result["steps"]:
        if not step["decided_mb_s"]:
            invalid.append(
                f"the {fmt_num(step['rate_mb_s'])} MB/s step decided nothing"
            )
    load = result["load"]
    if load["included"] <= 0:
        invalid.append("no transactions included")
    stakes = result["stake_table"]
    if len(stakes) != n_nodes or len(set(stakes)) != 1:
        invalid.append(f"stake table is not {n_nodes} equal entries: {stakes}")
    if result["host"]["steal_pct"] > STEAL_NOISY_PCT:
        noisy.append(
            f"steal {result['host']['steal_pct']:.1f}% > {STEAL_NOISY_PCT:.0f}%"
        )
    drift = result["calibration"]["drift_pct"]
    if abs(drift) > DRIFT_NOISY_PCT:
        noisy.append(f"calibration drift {drift:+.1f}% beyond {DRIFT_NOISY_PCT:.0f}%")
    tracker = load["tracker_lag_ms"]
    if tracker and tracker["p99"] > TRACKER_LAG_NOISY_MS:
        noisy.append(
            f"benchmark tracker behind: payload scans lag node0 by {tracker['p99']:.0f} ms p99"
        )
    if load["submit_errors"]:
        noisy.append(f"{load['submit_errors']} submit errors")
    if load["missing_payloads"]:
        noisy.append(f"query node lost payloads at heights {load['missing_payloads']}")
    return {"valid": not invalid, "noisy": bool(noisy), "reasons": invalid + noisy}


def run_meta(run: dict[str, Any]) -> RunMeta:
    meta = run["meta"]
    return {
        "sha": meta["sha"],
        "ref": meta["ref"],
        "event": meta["event"],
        "run_id": meta["run_id"],
        "run_url": meta["run_url"],
        "pr": meta["pr"],
        "started_at": datetime.fromtimestamp(run["started"], UTC).isoformat(
            timespec="seconds"
        ),
        "wall_s": run["wall_s"],
        "ready_s": run["ready_s"],
        "local": meta["local"],
        "teardown": run["teardown"],
    }


def config_hash(cfg: BenchConfig, extra: list[bytes]) -> str:
    data = dataclasses.asdict(cfg)
    # Runs from before `keep_going` existed hashed without it and stay comparable.
    if not cfg.keep_going:
        del data["keep_going"]
    digest = hashlib.sha256(json.dumps(data, sort_keys=True).encode())
    for chunk in extra:
        digest.update(chunk)
    return digest.hexdigest()[:12]


def load_series(path: Path, nodes: list[str]) -> dict[str, list[Sample]]:
    series: dict[str, list[Sample]] = {node: [] for node in nodes}
    for rec in read_jsonl(path):
        if rec["ok"]:
            series[rec["node"]].append((rec["ts"], rec["m"]))
    return series


def scrape_coverage(
    path: Path, nodes: list[str], t0: float, t1: float
) -> dict[str, float]:
    seen: dict[str, list[bool]] = {node: [] for node in nodes}
    for rec in read_jsonl(path):
        if t0 <= rec["ts"] <= t1:
            seen[rec["node"]].append(rec["ok"])
    return {node: sum(oks) / len(oks) if oks else 0.0 for node, oks in seen.items()}


def at(samples: list[Sample], t: float) -> Sample:
    """The last sample at or before `t`, else the first one."""
    best = samples[0]
    for sample in samples:
        if sample[0] > t:
            break
        best = sample
    return best


def delta(samples: list[Sample], ta: float, tb: float, key: str) -> tuple[float, float]:
    (ts0, m0), (ts1, m1) = at(samples, ta), at(samples, tb)
    return m1.get(key, 0.0) - m0.get(key, 0.0), ts1 - ts0


def rate(samples: list[Sample], ta: float, tb: float, key: str) -> float:
    d, dt = delta(samples, ta, tb, key)
    return d / dt if dt > 0 else 0.0


def node_stats(
    node: str, role: str, series: dict[str, list[Sample]], t0: float, t1: float
) -> NodeStats:
    samples = series[node]
    if not samples:
        return {
            "role": role,
            "decided_height_end": 0,
            "decided_blocks": 0,
            "view_lag_end": 0,
            "cpu_cores": 0.0,
            "rss_peak_bytes": 0,
            "tokio_busy_frac": None,
            "ops": {},
        }
    m0, m1 = at(samples, t0)[1], at(samples, t1)[1]
    top_view = max(
        at(s, t1)[1].get("consensus_last_decided_view", 0) for s in series.values() if s
    )
    in_window = [m for ts, m in samples if t0 <= ts <= t1] or [m1]
    workers = m1.get("tokio_workers")
    busy = rate(samples, t0, t1, "tokio_worker_busy_seconds_total")
    return {
        "role": role,
        "decided_height_end": int(m1.get("consensus_last_synced_block_height", 0)),
        "decided_blocks": int(
            delta(samples, t0, t1, "consensus_finalized_bytes_count")[0]
        ),
        "view_lag_end": int(top_view - m1.get("consensus_last_decided_view", 0)),
        "cpu_cores": rate(samples, t0, t1, "process_cpu_seconds_total"),
        "rss_peak_bytes": int(
            max(m.get("process_resident_memory_bytes", 0) for m in in_window)
        ),
        "tokio_busy_frac": busy / workers if workers else None,
        "ops": op_stats(m0, m1, delta(samples, t0, t1, "process_cpu_seconds_total")[1]),
    }


def op_stats(
    m0: dict[str, float], m1: dict[str, float], dt: float
) -> dict[str, OpStats]:
    ops: dict[str, OpStats] = {}
    for key, end in m1.items():
        if not key.endswith("_count") or "{" in key:
            continue
        base = key.removesuffix("_count")
        if base + "_sum" not in m1 or not is_duration(base):
            continue
        count = end - m0.get(key, 0.0)
        if count <= 0 or dt <= 0:
            continue
        total = m1[base + "_sum"] - m0.get(base + "_sum", 0.0)
        quantiles = histogram_quantiles([(m0, m1)], base)
        ops[base] = {
            "per_s": count / dt,
            "mean_ms": total / count * 1000,
            "p50_ms": quantiles["p50"] if quantiles else None,
            "p99_ms": quantiles["p99"] if quantiles else None,
            "busy_frac": total / dt,
        }
    return ops


def is_duration(base: str) -> bool:
    return base.endswith("_duration") or base.startswith("consensus_storage_")


def histogram_quantiles(
    pairs: list[tuple[dict[str, float], dict[str, float]]], base: str
) -> Quantiles | None:
    """Quantiles in ms of a seconds histogram over window deltas, summed across `pairs`, with
    linear interpolation inside a bucket as Prometheus' histogram_quantile does."""
    buckets: dict[float, float] = {}
    count = total = 0.0
    for m0, m1 in pairs:
        for key, value in m1.items():
            if key.startswith(base + "_bucket{") and 'le="' in key:
                le = key.split('le="')[1].split('"')[0]
                bound = math.inf if le == "+Inf" else float(le)
                buckets[bound] = buckets.get(bound, 0.0) + value - m0.get(key, 0.0)
        count += m1.get(base + "_count", 0.0) - m0.get(base + "_count", 0.0)
        total += m1.get(base + "_sum", 0.0) - m0.get(base + "_sum", 0.0)
    if count <= 0 or not buckets:
        return None
    bounds = sorted(buckets)
    finite_top = max((b for b in bounds if math.isfinite(b)), default=0.0)

    def quantile(q: float) -> float:
        rank = q * buckets[bounds[-1]]
        lower, below = 0.0, 0.0
        for bound in bounds:
            if buckets[bound] >= rank:
                if not math.isfinite(bound):
                    return finite_top * 1000
                inside = buckets[bound] - below
                frac = (rank - below) / inside if inside else 1.0
                return (lower + (bound - lower) * frac) * 1000
            lower, below = bound, buckets[bound]
        return finite_top * 1000

    top = next(b for b in bounds if buckets[b] >= buckets[bounds[-1]])
    return {
        "n": int(count),
        "mean": total / count * 1000,
        "p50": quantile(0.5),
        "p95": quantile(0.95),
        "p99": quantile(0.99),
        "max": (top if math.isfinite(top) else finite_top) * 1000,
    }


def window(
    series: dict[str, list[Sample]], query_node: str, t0: float, t1: float
) -> Window:
    samples = series[query_node]
    key = "consensus_last_synced_block_height"
    return {
        "t0": t0,
        "t1": t1,
        "height_start": int(at(samples, t0)[1].get(key, 0)) if samples else 0,
        "height_end": int(at(samples, t1)[1].get(key, 0)) if samples else 0,
    }


def process_stats(
    host: list[dict[str, Any]], t0: float, t1: float
) -> dict[str, ProcStats]:
    inside = [r for r in host if t0 <= r["ts"] <= t1]
    if len(inside) < 2:
        return {}
    first, last = inside[0], inside[-1]
    dt = last["ts"] - first["ts"]
    out: dict[str, ProcStats] = {}
    for label, now in sorted(last["procs"].items()):
        cpu = now["cpu_s"] - first["procs"].get(label, {"cpu_s": 0.0})["cpu_s"]
        out[label] = {
            "cpu_cores_mean": cpu / dt,
            "cpu_s": cpu,
            "rss_peak_bytes": max(
                r["procs"].get(label, {"rss": 0})["rss"] for r in inside
            ),
        }
    return out


def host_stats(host: list[dict[str, Any]], t0: float, t1: float) -> HostStats:
    inside = [r for r in host if t0 <= r["ts"] <= t1]
    if len(inside) < 2:
        return {
            "util_mean": 0.0,
            "util_max": 0.0,
            "steal_pct": 0.0,
            "iowait_pct": 0.0,
            "mem_avail_min_bytes": 0,
        }

    def busy(a: list[int], b: list[int]) -> tuple[float, float, float, float]:
        d = [y - x for x, y in zip(a, b)]
        total = sum(d[:8]) or 1
        idle, iowait, steal = d[3], d[4], d[7]
        return (total - idle - iowait) / total, steal / total, iowait / total, total

    steps = [busy(a["cpu"], b["cpu"]) for a, b in itertools.pairwise(inside)]
    whole = busy(inside[0]["cpu"], inside[-1]["cpu"])
    return {
        "util_mean": whole[0],
        "util_max": max(s[0] for s in steps),
        "steal_pct": whole[1] * 100,
        "iowait_pct": whole[2] * 100,
        "mem_avail_min_bytes": min(r["mem_avail"] for r in inside),
    }


def load_stats(
    out: Path,
    txs: list[dict[str, Any]],
    heights: list[dict[str, Any]],
    t0: float,
    t1: float,
) -> LoadStats:
    meta = read_json(out / "load-meta.json")
    return {
        "submitted": len(txs),
        "included": sum(1 for tx in txs if tx["status"] == "included"),
        "timeouts": sum(1 for tx in txs if tx["status"] == "timeout"),
        "submit_errors": meta["submit_errors"],
        "max_in_flight": meta["max_in_flight"],
        "cap_waits": meta["cap_waits"],
        "missing_payloads": meta["missing_payloads"],
        "tracker_lag_ms": quantiles(scan_lags(heights, t0, t1)),
        "drain_s": meta["drain_s"],
        "refine_skipped": meta["refine_skipped"],
    }


def scan_lags(heights: list[dict[str, Any]], t0: float, t1: float) -> list[float]:
    """Per height: ms from when its scan could start, once node0 showed the height
    PAYLOAD_LAG_BLOCKS above it, to the scan's end."""
    shown = {h["height"]: h["query"] for h in heights}
    lags = []
    for h in heights:
        ready = shown.get(h["height"] + PAYLOAD_LAG_BLOCKS)
        if ready is not None and h["scanned"] is not None and t0 <= ready <= t1:
            lags.append((h["scanned"] - ready) * 1000)
    return lags


def quantiles(values: list[float]) -> Quantiles | None:
    if not values:
        return None
    ordered = sorted(values)

    def q(p: float) -> float:
        return ordered[min(len(ordered) - 1, int(p * len(ordered)))]

    return {
        "n": len(ordered),
        "mean": statistics.fmean(ordered),
        "p50": q(0.5),
        "p95": q(0.95),
        "p99": q(0.99),
        "max": ordered[-1],
    }


def stake_amounts(path: Path) -> list[str]:
    table = read_json(path)["stake_table"]
    return [entry["stake_table_entry"]["stake_amount"] for entry in table]


def pct_change(before: float, after: float) -> float:
    return (after - before) / before * 100 if before else 0.0


@dataclass(frozen=True)
class Metric:
    label: str
    path: str
    unit: str
    better: Literal["higher", "lower"]
    floor_pct: float


CAPACITY_LIMITS = (
    ("capacity", "overall"),
    ("consensus limit", "consensus"),
    ("query node limit", "query_node"),
)
# Paths into one step of result.json's `steps`.
STEP_METRICS = (
    Metric("decided", "decided_mb_s", "MB/s", "higher", 10),
    Metric("consensus p50", "consensus_latency_ms.p50", "ms", "lower", 15),
    Metric("consensus p99", "consensus_latency_ms.p99", "ms", "lower", 20),
    Metric("query lag p50", "query_lag_ms.p50", "ms", "lower", 30),
    Metric("query lag p99", "query_lag_ms.p99", "ms", "lower", 30),
    Metric("e2e p50", "latency_ms.p50", "ms", "lower", 15),
    Metric("e2e p99", "latency_ms.p99", "ms", "lower", 20),
    Metric("view time", "mean_view_ms", "ms", "lower", 10),
    Metric("CPU-s/MB", "cpu_s_per_mb", "s/MB", "lower", 10),
    Metric("node0 CPU", "node_cpu.node0", "cores", "lower", 15),
    Metric("node1 CPU", "node_cpu.node1", "cores", "lower", 15),
    Metric("node2 CPU", "node_cpu.node2", "cores", "lower", 15),
    Metric("postgres CPU", "postgres_cpu", "cores", "lower", 15),
)
# The per-step summary table; the rest go to the step details.
STEP_COLUMNS = (
    "decided",
    "consensus p50",
    "consensus p99",
    "query lag p50",
    "e2e p50",
    "CPU-s/MB",
)


def lookup(result: Any, path: str) -> float | None:
    for part in path.split("."):
        if not isinstance(result, dict) or part not in result or result[part] is None:
            return None
        result = result[part]
    return float(result)


def compare(current: BenchResult, baseline: Baseline) -> Comparison:
    """Deltas against the median of comparable runs: capacity, and each step's metrics
    against the runs that ran the same rate. A delta counts only beyond both the metric's
    floor and 3 times the baseline's robust spread; a noisy current run is never conclusive."""
    usable: list[BenchResult] = []
    excluded: dict[str, int] = {}
    for run in baseline["runs"]:
        reason = exclusion(current, run)
        if reason:
            excluded[reason] = excluded.get(reason, 0) + 1
        else:
            usable.append(run)
    usable = usable[:BASELINE_RUNS]
    noisy = current["validity"]["noisy"]
    resolution = ramp_resolution(current["config"]["steps"])
    capacity_rows = [
        compare_capacity(
            label,
            current["capacity"][key],
            [run["capacity"][key] for run in usable],
            resolution,
            noisy,
        )
        for label, key in CAPACITY_LIMITS
    ]
    rates = sorted(
        {step["rate_mb_s"] for run in [current, *usable] for step in run["steps"]}
    )
    return {
        "n": len(usable),
        "excluded": excluded,
        "error": baseline["error"],
        "source": baseline["source"],
        "used": [run["run"] for run in usable],
        "capacity": capacity_rows,
        "steps": [compare_step(rate, current, usable, noisy) for rate in rates],
    }


def ramp_resolution(steps: Sequence[float]) -> float:
    """The smallest capacity change the ramp can show: a refine step, half a step gap."""
    gaps = [b - a for a, b in itertools.pairwise(steps)]
    return min(gaps) / 2 if gaps else steps[0] / 2


def compare_capacity(
    label: str, current: Limit, history: list[Limit], resolution: float, noisy: bool
) -> CapacityRow:
    """In MB/s, not percent: capacity moves in ramp steps. A capacity below the first step
    ranks lowest; a lower bound (no step failed) below the baseline says nothing."""
    ordered = sorted(
        history, key=lambda lim: -math.inf if lim["mb_s"] is None else lim["mb_s"]
    )
    baseline = ordered[(len(ordered) - 1) // 2] if ordered else None
    cur = current["mb_s"]
    base = baseline["mb_s"] if baseline else None
    delta = cur - base if cur is not None and base is not None else None
    if baseline is None:
        verdict = "n/a"
    elif noisy:
        verdict = "inconclusive"
    elif cur is None or base is None:
        verdict = "same" if cur == base else ("worse" if cur is None else "better")
    elif delta is not None and abs(delta) < resolution - 1e-9:
        verdict = "same"
    elif delta is not None and delta < 0:
        verdict = "worse" if current["bounded"] else "inconclusive"
    else:
        verdict = "better" if baseline["bounded"] else "inconclusive"
    return {
        "label": label,
        "current": current,
        "baseline": baseline,
        "n": len(history),
        "delta_mb_s": delta,
        "resolution_mb_s": resolution,
        "verdict": verdict,
    }


def compare_step(
    rate: float, current: BenchResult, usable: list[BenchResult], noisy: bool
) -> StepComparison:
    mine = step_at(current, rate)
    theirs = [step for run in usable if (step := step_at(run, rate))]
    return {
        "rate_mb_s": rate,
        "n": len(theirs),
        "rows": [
            compare_metric(
                metric,
                lookup(mine, metric.path),
                [lookup(step, metric.path) for step in theirs],
                noisy,
            )
            for metric in STEP_METRICS
        ],
    }


def step_at(result: BenchResult, rate: float) -> StepResult | None:
    return next((s for s in result["steps"] if s["rate_mb_s"] == rate), None)


def exclusion(current: BenchResult, run: BenchResult) -> str | None:
    """Why `run` is no baseline for `current`, or None if it is one."""
    if run.get("schema_version") != SCHEMA_VERSION:
        return "other schema"
    if not run["validity"]["valid"]:
        return "invalid"
    if run["validity"]["noisy"]:
        return "noisy"
    if run["config_hash"] != current["config_hash"]:
        return "other config"
    ours, theirs = current["runner"], run["runner"]
    if (ours["cpu_model"], ours["affinity"]) != (
        theirs["cpu_model"],
        theirs["affinity"],
    ):
        return "other runner"
    speed = pct_change(
        current["calibration"]["before"]["sha256_1t_mb_s"],
        run["calibration"]["before"]["sha256_1t_mb_s"],
    )
    if abs(speed) > CALIBRATION_MATCH_PCT:
        return "other calibration"
    return None


def compare_metric(
    metric: Metric, value: float | None, values: list[float | None], noisy: bool
) -> Row:
    history = [v for v in values if v is not None]
    baseline = statistics.median(history) if history else None
    spread = 0.0
    if baseline and len(history) >= 3:
        mad = statistics.median(abs(v - baseline) for v in history)
        spread = 1.4826 * mad / abs(baseline) * 100
    threshold = max(metric.floor_pct, 3 * spread)
    delta_pct = pct_change(baseline, value) if baseline and value is not None else None
    if value is None or baseline is None:
        verdict = "n/a"
    elif noisy:
        verdict = "inconclusive"
    elif baseline == 0:
        verdict = (
            "same"
            if value == 0
            else ("worse" if metric.better == "lower" else "better")
        )
    elif delta_pct is not None and abs(delta_pct) > threshold:
        verdict = (
            "better" if (delta_pct > 0) == (metric.better == "higher") else "worse"
        )
    else:
        verdict = "same"
    return {
        "key": metric.path,
        "label": metric.label,
        "unit": metric.unit,
        "better": metric.better,
        "current": value,
        "baseline": baseline,
        "delta_pct": delta_pct,
        "threshold_pct": threshold,
        "verdict": verdict,
    }


def load_baseline(path: Path) -> Baseline:
    """`nextest-ci stats-fetch` output ({runs: [...], error?}, newest first) or a single
    result.json. A stats-fetch `error` means the lookup failed, not that nothing was published."""
    match read_json(path):
        case {"runs": list() as runs} as fetched:
            return {"runs": runs, "error": fetched.get("error"), "source": "main"}
        case {"schema_version": _} as single:
            return {"runs": [single], "error": None, "source": "reference"}
        case other:
            raise ValueError(
                f"{path} is neither stats-fetch output nor a result.json: {other!r:.200}"
            )


def render(result: BenchResult, comparison: Comparison | None) -> str:
    lines = [
        "## Network benchmark",
        "",
        f"- {status_line(result)}",
        f"- {capacity_line(result['capacity'])}",
        f"- {baseline_line(comparison)}",
        "",
        *capacity_table(result, comparison),
        "### Load steps",
        "",
        *step_table(result, comparison),
        "",
        *details("Step details", step_details(result)),
        "### Load",
        "",
        *load_lines(result),
        "",
        *details("Runner, calibration, host", runner_lines(result)),
        *(details("Deployment", body) if (body := deployment_lines(result)) else []),
        *(details("Hosts", body) if (body := hosts_table(result)) else []),
        *details("Nodes", node_table(result)),
        *details("Top ops by busy time", ops_table(result)),
        *details("Processes", process_table(result)),
        *footer(result),
    ]
    return "\n".join(lines) + "\n"


def status_line(result: BenchResult) -> str:
    validity = result["validity"]
    reasons = "; ".join(validity["reasons"])
    run = result["run"]
    head = f"`{run['sha'][:10]}` {run_where(run)}, config `{result['config_hash']}`"
    return f"{head}: run **{verdict_word(validity)}**" + (
        f" ({reasons})" if reasons else ""
    )


def run_where(run: RunMeta) -> str:
    # A dispatched PR run has the PR branch as ref; the PR number says more.
    return f"PR #{run['pr']}" if run["pr"] else f"{run['event']} {run['ref']}"


def baseline_label(comparison: Comparison) -> str:
    if comparison["source"] == "main":
        return f"main median (n={comparison['n']})"
    return f"reference `{comparison['used'][0]['sha'][:10]}`"


def baseline_line(comparison: Comparison | None) -> str:
    if comparison is None:
        return "Baseline: none given, no main runs to compare against."
    if comparison["error"]:
        return f"Baseline: fetching main runs failed: {comparison['error']}."
    note = excluded_note(comparison)
    if comparison["source"] == "reference":
        if not comparison["used"]:
            return f"Baseline: reference run not comparable{note}."
        run = comparison["used"][0]
        return f"Baseline: reference run {run_ref(run)}, {run_where(run)}."
    if not comparison["used"]:
        return f"Baseline: no main runs to compare against{note}."
    runs = ", ".join(run_ref(run) for run in comparison["used"])
    return f"Baseline: median of {comparison['n']} main runs: {runs}{note}."


def run_ref(run: RunMeta) -> str:
    sha = f"`{run['sha'][:10]}`"
    started = run["started_at"][:16].replace("T", " ") + " UTC"
    return (
        f"[{sha}]({run['run_url']}) {started}" if run["run_url"] else f"{sha} {started}"
    )


def verdict_word(validity: Validity) -> str:
    if not validity["valid"]:
        return "invalid"
    return "noisy" if validity["noisy"] else "valid"


def runner_lines(result: BenchResult) -> list[str]:
    r, c, h = result["runner"], result["calibration"], result["host"]
    mhz = statistics.fmean(r["mhz"]) if r["mhz"] else 0.0
    image = " ".join(x for x in (r["image_os"], r["image_version"]) if x) or "local"
    quota = f", cgroup cpu.max {r['cgroup_cpu_max']}" if r["cgroup_cpu_max"] else ""
    before, after = c["before"], c["after"]
    return [
        f"- CPU: {r['cpu_model']}, {mhz:.0f} MHz mean",
        f"- CPUs: {r['affinity']} usable of {r['nproc']}{quota}",
        f"- RAM: {fmt_bytes(r['mem_total_bytes'])}",
        f"- image: {image}",
        (
            f"- sha256 1 core: {before['sha256_1t_mb_s']:.0f} MB/s before, "
            f"{after['sha256_1t_mb_s']:.0f} MB/s after"
        ),
        (
            f"- sha256 all usable cores: {before['sha256_mt_mb_s']:.0f} MB/s before, "
            f"{after['sha256_mt_mb_s']:.0f} MB/s after, drift {c['drift_pct']:+.1f}%"
        ),
        f"- fsync: {before['fsync_per_s']:.0f}/s",
        f"- host CPU busy (usable CPUs): {h['util_mean']:.0%} mean, {h['util_max']:.0%} max",
        f"- steal {h['steal_pct']:.1f}%, iowait {h['iowait_pct']:.1f}%",
    ]


def deployment_lines(result: BenchResult) -> list[str]:
    """AWS fleet metadata and cost; empty for the local bench."""
    if "deployment" not in result:
        return []
    d = result["deployment"]
    images = ", ".join(f"{name} {ref}" for name, ref in sorted(d["images"].items()))
    rev = d["image_revision"]
    images_line = (
        f"- images @ {rev}: {images}" if rev is not None else f"- images: {images}"
    )
    types = ", ".join(
        f"{name} {meta['instance_type']}" for name, meta in sorted(d["hosts"].items())
    )
    cost = d["cost_usd"]
    cost_line = f"expected ${cost['expected']:.2f}, bound ${cost['bound']:.2f}"
    if "actual" in cost:
        cost_line += f", actual ${cost['actual']:.2f}"
    return [
        f"- account {d['account']} ({d['region']}), az {d['az']}, ami {d['ami']}",
        images_line,
        f"- instance types: {types}",
        (
            f"- start spread {d['start_spread_s']:.1f} s, clock offset max "
            f"{d['clock_offset_ms_max']:.0f} ms"
        ),
        f"- cost: {cost_line}",
        *([query_db_line(d["query_db"])] if "query_db" in d else []),
        *([f"- node env: {', '.join(d['node_env'])}"] if "node_env" in d else []),
    ]


def query_db_line(q: QueryDbMeta) -> str:
    store = q["store"]
    ident = f" {store['identifier']}" if "identifier" in store else ""
    cls = f" on {q['instance_class']}" if "instance_class" in q else ""
    return (
        f"- Query DB: {q['mode']}, {q['engine']}{cls}, {store['type']}{ident} "
        f"{store['gb']} GB {store['iops']} IOPS {store['mbps']} MB/s, "
        f"TLS {'on' if q['tls'] else 'off'}"
    )


def hosts_table(result: BenchResult) -> list[str]:
    """Per-host CPU, steal and I/O; empty when no per-host samples were collected."""
    if "hosts" not in result or not result["hosts"]:
        return []
    lines = [
        "| host | CPU busy mean | steal | disk MB/s | net MB/s |",
        "|---|---:|---:|---:|---:|",
    ]
    for name, h in sorted(result["hosts"].items()):
        lines.append(
            f"| {name} | {h['util_mean']:.0%} | {h['steal_pct']:.1f}% "
            f"| {fmt_num(h['disk_mb_s'])} | {fmt_num(h['net_mb_s'])} |"
        )
    return lines


def details(summary: str, body: list[str]) -> list[str]:
    """A collapsed block; GitHub renders markdown inside only after a blank line."""
    return [f"<details><summary>{summary}</summary>", "", *body, "", "</details>", ""]


def capacity_table(result: BenchResult, comparison: Comparison | None) -> list[str]:
    """Capacity against the baseline; nothing without one."""
    if comparison is None or not comparison["used"]:
        return []
    first = result["config"]["steps"][0]
    lines = [
        (
            f"| metric | this run | {baseline_label(comparison)} | runs | delta "
            "| verdict (resolution) |"
        ),
        "|---|---:|---:|---:|---:|---|",
    ]
    for row in comparison["capacity"]:
        delta = row["delta_mb_s"]
        mark = {"worse": "**worse**", "better": "better"}.get(
            row["verdict"], row["verdict"]
        )
        base = fmt_limit(row["baseline"], first) if row["baseline"] else ""
        lines.append(
            f"| {row['label']} | {fmt_limit(row['current'], first)} | {base} "
            f"| {row['n']} | {f'{delta:+g} MB/s' if delta is not None else ''} "
            f"| {mark} (±{fmt_num(row['resolution_mb_s'])} MB/s) |"
        )
    return [*lines, ""]


def fmt_limit(limit: Limit, first: float) -> str:
    if limit["mb_s"] is None:
        return f"< {fmt_num(first)} MB/s"
    bound = "" if limit["bounded"] else ">= "
    return f"{bound}{fmt_num(limit['mb_s'])} MB/s"


def step_table(result: BenchResult, comparison: Comparison | None) -> list[str]:
    """One row per rate this run ran, and per ramp rate the baseline ran but this run did not
    reach; a cell beyond its threshold against the baseline median at that rate carries the
    delta, bold when worse. Refine rates only the baseline ran are left out."""
    by_rate = (
        {c["rate_mb_s"]: c for c in comparison["steps"]}
        if comparison and comparison["used"]
        else {}
    )
    ramp = set(result["config"]["steps"])
    rates = sorted({s["rate_mb_s"] for s in result["steps"]} | (set(by_rate) & ramp))
    runs = (
        "main runs"
        if comparison and comparison["source"] == "main"
        else "reference runs"
    )
    lines = [
        "| MB/s | "
        + " | ".join(STEP_COLUMNS)
        + " | verdict |"
        + (f" {runs} |" if by_rate else ""),
        "|---:|" + "---:|" * len(STEP_COLUMNS) + "---|" + ("---:|" if by_rate else ""),
    ]
    for rate in rates:
        step, other = step_at(result, rate), by_rate.get(rate)
        tail = f" {other['n']} |" if other else (" 0 |" if by_rate else "")
        if step is None:
            lines.append(
                f"| {fmt_num(rate)} |"
                + " |" * len(STEP_COLUMNS)
                + f" **not reached** |{tail}"
            )
            continue
        rows = {row["label"]: row for row in other["rows"]} if other else {}
        cells = [step_cell(step, label, rows.get(label)) for label in STEP_COLUMNS]
        lines.append(
            f"| {fmt_num(rate)}{' (refine)' if step['refine'] else ''} | "
            + " | ".join(cells)
            + f" | {step_verdict(step)} |{tail}"
        )
    return lines


def step_cell(step: StepResult, label: str, row: Row | None) -> str:
    metric = next(m for m in STEP_METRICS if m.label == label)
    text = fmt_value(lookup(step, metric.path), metric.unit)
    if (
        row is None
        or row["delta_pct"] is None
        or row["verdict"] not in ("worse", "better")
    ):
        return text
    text += f" ({row['delta_pct']:+.0f}%)"
    return f"**{text}**" if row["verdict"] == "worse" else text


def step_verdict(step: StepResult) -> str:
    fails = step["consensus_fails"] + step["query_fails"]
    return "pass" if not fails else "fail: " + ", ".join(map(fail_code, fails))


# Rule text prefix -> the short name in the step table; see step_fails.
FAIL_CODES = (
    ("decided", "decided"),
    ("no consensus metrics", "no metrics"),
    ("consensus latency", "latency"),
    ("query lag grows", "query lag growth"),
    ("query lag p50", "query lag"),
)


def fail_code(rule: str) -> str:
    if rule.endswith("view timeouts"):
        return "timeouts"
    return next((code for prefix, code in FAIL_CODES if rule.startswith(prefix)), rule)


def step_details(result: BenchResult) -> list[str]:
    lines = [
        (
            "| MB/s | decided | view timeouts | consensus p50/p99 ms "
            "| query lag p50/p99 ms | query lag slope ms/s | e2e p50/p99 ms | view ms "
            "| CPU-s/MB | node0/1/2 CPU | postgres CPU | failed rules |"
        ),
        "|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for s in sorted(result["steps"], key=lambda s: s["rate_mb_s"]):
        cpu = "/".join(fmt_num(s["node_cpu"].get(node)) for node in result["nodes"])
        lines.append(
            f"| {fmt_num(s['rate_mb_s'])} | {fmt_num(s['decided_mb_s'])} "
            f"| {s['timeouts']} "
            f"| {pair(s['consensus_latency_ms'])} | {pair(s['query_lag_ms'])} "
            f"| {fmt_num(s['query_lag_slope_ms_s'])} | {pair(s['latency_ms'])} "
            f"| {fmt_num(s['mean_view_ms'])} | {fmt_num(s['cpu_s_per_mb'])} | {cpu} "
            f"| {fmt_num(s['postgres_cpu'])} "
            f"| {'; '.join(s['consensus_fails'] + s['query_fails'])} |"
        )
    return lines


def pair(q: Quantiles | None) -> str:
    return f"{q['p50']:.0f}/{q['p99']:.0f}" if q else ""


def excluded_note(comparison: Comparison) -> str:
    excluded = comparison["excluded"]
    return (
        " (excluded: " + ", ".join(f"{n} {why}" for why, n in excluded.items()) + ")"
        if excluded
        else ""
    )


def node_table(result: BenchResult) -> list[str]:
    lines = [
        "| node | role | decided height | view lag | tokio workers busy |",
        "|---|---|---:|---:|---:|",
    ]
    for node, s in result["nodes"].items():
        busy = f"{s['tokio_busy_frac']:.0%}" if s["tokio_busy_frac"] is not None else ""
        lines.append(
            f"| {node} | {s['role']} | {s['decided_height_end']} | {s['view_lag_end']} "
            f"| {busy} |"
        )
    return lines


# Timers nested in another listed op: their numbers repeat the outer one's.
DUPLICATE_OPS = frozenset(
    {
        "consensus_decide_processor_process_duration",
        "consensus_internal_append_da2_duration",
        "consensus_internal_append_vid_duration",
        "consensus_internal_append_quorum2_duration",
    }
)


def ops_table(result: BenchResult, count: int = 8) -> list[str]:
    """The ops that kept any node busiest, as p99 and busy fraction per node."""
    busiest: dict[str, float] = {}
    for s in result["nodes"].values():
        for op, stats in s["ops"].items():
            if op not in DUPLICATE_OPS:
                busiest[op] = max(busiest.get(op, 0.0), stats["busy_frac"])
    top = sorted(busiest, key=lambda op: -busiest[op])[:count]
    nodes = list(result["nodes"])
    lines = [
        "| op (p99 ms, busy %) | " + " | ".join(nodes) + " |",
        "|---|" + "---:|" * len(nodes),
    ]
    for op in top:
        cells = []
        for node in nodes:
            stats = result["nodes"][node]["ops"].get(op)
            cells.append(
                f"{fmt_num(stats['p99_ms'])}, {stats['busy_frac']:.0%}" if stats else ""
            )
        lines.append(f"| {op.removeprefix('consensus_')} | " + " | ".join(cells) + " |")
    return lines + [
        "",
        "busy: summed op duration over the window length; above 100% means overlapping calls.",
    ]


def process_table(result: BenchResult) -> list[str]:
    lines = [
        "| process | CPU cores mean | CPU-s | RSS peak (sum over its PIDs) |",
        "|---|---:|---:|---:|",
    ]
    for label, p in sorted(result["processes"].items(), key=lambda kv: -kv[1]["cpu_s"]):
        lines.append(
            f"| {label} | {p['cpu_cores_mean']:.2f} | {p['cpu_s']:.0f} "
            f"| {fmt_bytes(p['rss_peak_bytes'])} |"
        )
    return lines


def load_lines(result: BenchResult) -> list[str]:
    load, cfg = result["load"], result["config"]
    return [
        (
            f"- steps: {', '.join(map(fmt_num, cfg['steps']))} MB/s, {cfg['step_s']} s each, "
            f"second half measured, after {cfg['warmup_s']} s warmup; "
            f"{fmt_bytes(cfg['tx_size'])} txs to {cfg['submit_nodes']} nodes, "
            f"{cfg['workers']} submit threads, in flight capped at {fmt_num(cfg['cap_s'])} s "
            "of load"
        ),
        (
            f"- step rules: decided >= {KEEP_UP_RATIO:.0%} of offered, no view timeouts, consensus latency p50 "
            f"<= {cfg['latency_target_ms']} ms; query lag p50 <= "
            f"{cfg['query_lag_target_ms']} ms, growth <= {QUERY_LAG_GROWTH_MS_S:.0f} ms/s"
        ),
        (
            f"- {load['submitted']} submitted, {load['included']} included, "
            f"{load['timeouts']} timed out, {load['submit_errors']} submit errors, "
            f"peak in flight {load['max_in_flight']}, {load['cap_waits']} submits waited"
        ),
        f"- benchmark tracker lag: {spread(load['tracker_lag_ms'])}",
        *(
            ["- refine step skipped: the failed step's backlog did not drain"]
            if load["refine_skipped"]
            else []
        ),
        *drain_lines(load["drain_s"], cfg["keep_going"]),
    ]


def drain_lines(drain_s: float | None, keep_going: bool) -> list[str]:
    if drain_s is not None:
        when = "after the last step" if keep_going else "before the refine step"
        return [f"- backlog drained in {drain_s:.1f} s {when}"]
    if keep_going:
        return [f"- backlog did not drain in {CATCHUP_TIMEOUT_S} s after the last step"]
    return []


def spread(q: Quantiles | None) -> str:
    if q is None:
        return "n/a"
    return (
        f"p50 {q['p50']:.0f}, p95 {q['p95']:.0f}, p99 {q['p99']:.0f}, "
        f"max {q['max']:.0f} ms (n={q['n']})"
    )


def footer(result: BenchResult) -> list[str]:
    w, run = result["window"], result["run"]
    lines = [
        (
            f"Window {w['t1'] - w['t0']:.0f} s, heights {w['height_start']} to "
            f"{w['height_end']}; network ready after {run['ready_s']:.0f} s, run took "
            f"{run['wall_s']:.0f} s."
        )
    ]
    if run["teardown"]:
        lines.append("Teardown: " + "; ".join(run["teardown"]) + ".")
    if run["run_url"]:
        lines.append(f"Raw data: artifacts of [this run]({run['run_url']}).")
    return lines


def render_failure(error: str, runner: RunnerInfo) -> str:
    return (
        f"## Network benchmark\n\n**invalid**: {error}\n\nRunner: {runner['cpu_model']}, "
        f"{runner['nproc']} CPUs. Logs are in the artifact.\n"
    )


def fmt_value(value: float | None, unit: str) -> str:
    if value is None:
        return ""
    if unit == "bytes":
        return fmt_bytes(value)
    sep = "" if unit.startswith("/") else " "
    return f"{fmt_num(value)}{sep}{unit}".strip()


def fmt_num(value: float | None) -> str:
    if value is None:
        return ""
    if value == 0 or abs(value) >= 100:
        return f"{value:.0f}"
    return f"{value:.3g}"


def fmt_bytes(value: float) -> str:
    for unit in ("B", "KB", "MB", "GB"):
        if abs(value) < 1000:
            return f"{value:.0f} {unit}" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1000
    return f"{value:.1f} TB"


def parse_prom(text: str) -> dict[str, float]:
    out = {}
    for line in text.splitlines():
        if not line or line.startswith("#"):
            continue
        key, _, value = line.rpartition(" ")
        try:
            out[key] = float(value)
        except ValueError:
            continue
    return out


def read_json(path: Path) -> Any:
    return json.loads(path.read_text())


def write_json(path: Path, data: Any) -> None:
    """Strict JSON: NaN and Infinity are not JSON and break readers of result.json."""
    path.write_text(json.dumps(data, indent=2, allow_nan=False) + "\n")


def read_jsonl(path: Path) -> Iterator[dict[str, Any]]:
    with open(path) as f:
        for line in f:
            yield json.loads(line)


def write_jsonl(path: Path, records: Iterator[dict[str, Any]]) -> None:
    with open(path, "w") as f:
        f.writelines(json.dumps(record, allow_nan=False) + "\n" for record in records)
