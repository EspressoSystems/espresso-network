"""A null node for measuring the load driver's own ceiling without a network.

It accepts `/v1/submit/submit` and makes a block of the received transactions every
`BLOCK_S`: a payload with each transaction's header (marker and id) followed by zeros up to
the transaction's size, so the driver's payload scans fetch, decode and search as much data
as against a real query node. Each transaction is padded to a multiple of 3 bytes, so a block
is streamed from the headers' base64 and one pre-encoded filler: the server does no encoding. It answers the height, metrics and payload endpoints the load
generator reads. Local use only (`bench selftest`); not shipped to AWS hosts.
"""

import asyncio
import base64
import multiprocessing
import os
import re
import tempfile
import threading
import time
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from multiprocessing.synchronize import Event
from pathlib import Path

import netbench as nb

BLOCK_S = 1.0
# Blocks kept for the scans: the driver reads each one a block behind the newest.
BLOCKS_KEPT = 8
HEADER_LEN = 24
PAYLOAD_KEY = b'"payload":"'
HEADER_B64 = HEADER_LEN // 3 * 4


class NullNode:
    """State shared by the request threads and the block thread. A block is the list of its
    transactions' base64 headers."""

    def __init__(self, tx_size: int) -> None:
        self.tx_bytes = -(-tx_size // 3) * 3
        self.filler = base64.b64encode(bytes(self.tx_bytes - HEADER_LEN))
        self.lock = threading.Lock()
        self.pending: list[bytes] = []
        self.blocks: dict[int, list[bytes]] = {}
        self.count = 0
        self.decided_bytes = 0

    def submit(self, body: bytes) -> None:
        start = body.index(PAYLOAD_KEY) + len(PAYLOAD_KEY)
        with self.lock:
            self.pending.append(body[start : start + HEADER_B64])

    def make_block(self) -> None:
        with self.lock:
            headers, self.pending = self.pending, []
            self.blocks[self.count] = headers
            self.blocks.pop(self.count - BLOCKS_KEPT, None)
            self.count += 1
            self.decided_bytes += len(headers) * self.tx_bytes

    def payload_parts(self, height: int) -> list[bytes] | None:
        """The pieces of the JSON answer for block `height`, None if it is gone."""
        with self.lock:
            headers = self.blocks.get(height)
        if headers is None:
            return None
        parts = [b'{"data":{"raw_payload":"']
        for header in headers:
            parts += [header, self.filler]
        parts.append(b'"}}')
        return parts

    def block_loop(self, stop: threading.Event) -> None:
        while not stop.wait(BLOCK_S):
            self.make_block()

    def get(self, path: str) -> tuple[int, bytes]:
        with self.lock:
            if path == "/v1/node/block-height":
                return 200, str(self.count).encode()
            if path == "/v1/status/block-height":
                return 200, str(self.count - 1).encode()
            if path == "/v1/status/metrics":
                text = (
                    f"consensus_finalized_bytes_sum {self.decided_bytes}\n"
                    "consensus_number_of_timeouts 0\n"
                )
                return 200, text.encode()
        return 404, b""


def handler_for(node: NullNode) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, format: str, *args: object) -> None:
            pass

        def reply(self, status: int, *parts: bytes) -> None:
            self.send_response(status)
            self.send_header("Content-Length", str(sum(map(len, parts))))
            self.end_headers()
            for part in parts:
                self.wfile.write(part)

        def do_GET(self) -> None:
            if match := re.fullmatch(r"/v1/availability/payload/(\d+)", self.path):
                parts = node.payload_parts(int(match.group(1)))
                self.reply(404, b"") if parts is None else self.reply(200, *parts)
            else:
                self.reply(*node.get(self.path))

        def do_POST(self) -> None:
            node.submit(self.rfile.read(int(self.headers["Content-Length"])))
            self.reply(200, b"{}")

    return Handler


def serve(
    tx_size: int,
    port: "multiprocessing.Queue[int]",
    stop: Event,
    cpu_s: "multiprocessing.Queue[float]",
) -> None:
    """Runs until `stop` is set; reports the port it bound, then its CPU seconds."""
    node = NullNode(tx_size)
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler_for(node))
    server.daemon_threads = True
    halt = threading.Event()
    threading.Thread(target=node.block_loop, args=(halt,), daemon=True).start()
    threading.Thread(target=server.serve_forever, daemon=True).start()
    port.put(server.server_address[1])
    stop.wait()
    halt.set()
    server.shutdown()
    cpu_s.put(time.process_time())


@dataclass
class SelfTest:
    target_mb_s: float
    submitted_mb_s: float
    queued_mb_s: float | None
    queue_wait_p99_ms: float | None
    cap_waits: int
    sent: int
    included: int
    driver_cores: float
    scan_cores: float
    server_cores: float


def run_selftest(
    rate_mb_s: float,
    step_s: int,
    workers: int,
    out: Path | None = None,
    tx_size: int = 1_000_000,
    scan_processes: int = nb.SCAN_PROCESSES,
) -> SelfTest:
    """One step of `step_s` seconds at `rate_mb_s` against a null node in its own process.
    CPU is the driver process's and its scan processes', over the whole run including the
    body pool's creation, divided by the run's wall time."""
    context = multiprocessing.get_context("spawn")
    port, cpu_s, stop = context.Queue(), context.Queue(), context.Event()
    server = context.Process(target=serve, args=(tx_size, port, stop, cpu_s))
    server.start()
    url = f"http://127.0.0.1:{port.get(timeout=30)}"
    cfg = nb.BenchConfig(
        tx_size=tx_size,
        steps=(rate_mb_s,),
        step_s=step_s,
        warmup_s=0,
        workers=workers,
        submit_nodes=1,
        tx_timeout_s=10,
    )
    with tempfile.TemporaryDirectory() as scratch:
        directory = out or Path(scratch)
        directory.mkdir(parents=True, exist_ok=True)
        start, before = time.monotonic(), os.times()
        try:
            asyncio.run(
                nb.generate_load(
                    cfg,
                    [url],
                    [url],
                    [url],
                    directory,
                    nb.SYSTEM_CLOCK,
                    nb.HttpPool,
                    scan_processes,
                )
            )
            wall, after = time.monotonic() - start, os.times()
            stop.set()
            server_cpu = cpu_s.get(timeout=30)
        finally:
            stop.set()
            server.join(timeout=30)
        step = nb.read_json(directory / "steps.json")[0]
        meta = nb.read_json(directory / "load-meta.json")
        txs = list(nb.read_jsonl(directory / "load.jsonl"))
    wait = step["queue_wait_ms"]
    return SelfTest(
        target_mb_s=rate_mb_s,
        submitted_mb_s=step["submitted_mb_s"],
        queued_mb_s=step["queued_mb_s"],
        queue_wait_p99_ms=wait["p99"] if wait else None,
        cap_waits=meta["cap_waits"],
        sent=len(txs),
        included=sum(1 for tx in txs if tx["status"] == "included"),
        driver_cores=(after.user + after.system - before.user - before.system) / wall,
        scan_cores=(
            after.children_user
            + after.children_system
            - before.children_user
            - before.children_system
        )
        / wall,
        server_cores=server_cpu / wall,
    )


def report(result: SelfTest, cpu_model: str, workers: int) -> str:
    return (
        f"target {result.target_mb_s:g} MB/s: submitted {result.submitted_mb_s:.1f}, "
        f"queued {nb.fmt_num(result.queued_mb_s)}, queue wait p99 "
        f"{nb.fmt_num(result.queue_wait_p99_ms)} ms, {result.cap_waits} cap waits, "
        f"{result.included} of {result.sent} txs seen in blocks\n"
        f"driver CPU {result.driver_cores:.2f} cores, scan processes "
        f"{result.scan_cores:.2f}, null server {result.server_cores:.2f} "
        f"({workers} submit workers; {cpu_model}); the null node decides nothing, so "
        "consensus and query verdicts do not apply"
    )


def cpu_model() -> str:
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.startswith("model name"):
            return line.partition(":")[2].strip()
    raise RuntimeError("no model name in /proc/cpuinfo")
