# The VID reconstruction window

How long a validator may take to reconstruct a block before consensus feels it, from a model with every computation
instant, and what the October 2026 bench runs showed next to it. The model is `scripts/network-bench/vidwindow.py`; the
comparison with runs is `vidwindow_runs.py` beside it.

## What reconstruction gates

A node validates proposal V+1 once it holds the proposal and its own share of block V+1, and casts vote1 on V+1 only
once it has reconstructed block V (`maybe_vote_1` in `crates/hotshot/new-protocol/src/consensus.rs`). Nothing else waits
for the payload: the next leader builds its block and disperses the shares as soon as it validates the current proposal,
and proposes once it holds Cert1 (`maybe_propose` checks the certificate, the header and its own built block; the traces
confirm the proposal leaves within a millisecond of Cert1 even when reconstruction ends 75 ms later).

So the window of a node is the time from its k-th share of block V to its validation of proposal V+1. Only the q-th
vote1 to reach the next leader sets Cert1, so the window that matters per view is the largest reconstruction time,
applied at every node, that leaves that q-th vote where it is. The critical voter is normally a far node that received
the proposal late and had a long window; near nodes with short windows vote early and are not on the critical path.

## The model

One interface per node of capacity B in each direction, flows sharing it fairly, in-order delivery per connection,
one-way delays from `latency.py`. With T the share transfer time, (n - 1) / k × block ÷ bandwidth, the window for a
given leader is

    window ≈ max(Q − T, T + δ)

- Q is the quorum round trip, the q-th smallest d(L, j) + d(j, L') over voters j for the leader L of V and the leader L'
  of V+1, read straight from the latency matrix. For 100 nodes placed as decaf-2025 places them it is about 153 ms from
  eu-central-1 or ap-southeast-1, 182 ms from us-east-1, 312 ms from sa-east-1.
- δ is the latency left once transfers dominate, the detour the proposal takes through the next leader relative to the
  shares' direct paths: one intra-region hop (5 ms) for a leader in the majority region, 25 to 128 ms elsewhere. It
  comes from the simulation.
- The minimum over bandwidth is (Q + δ) / 2, independent of block size, at bandwidth 2 (n - 1) / k × block ÷ (Q − δ).
  Below that bandwidth the view is transfer-bound (two share transfers per view) and the window is T + δ; above it the
  window climbs back toward Q.

For 100 nodes and the f+1 threshold (34 shares, each node moving 2.9 × the block per view, 23 ms per MB per Gbps):

| leader region  | Q ms | δ ms | minimum window | at Gbps per MB of block |
| -------------- | ---- | ---- | -------------- | ----------------------- |
| eu-central-1   | 153  | 5    | 79 ms          | 0.31                    |
| ap-southeast-1 | 153  | 61   | 107 ms         | 0.51                    |
| us-east-1      | 182  | 25   | 103 ms         | 0.30                    |
| ap-southeast-2 | 157  | 53   | 105 ms         | 0.45                    |
| sa-east-1      | 312  | 128  | 220 ms         | 0.25                    |

Without latency everything collapses to one line: window = T, view = 2T, throughput = bandwidth ÷ 5.8 whatever the block
size (417 MB/s with 10 nodes at 15 Gbps, where the bench measured a 335 MB/s cliff; the gap is TCP and message overhead
the model leaves out).

**2f+1 threshold** (67 shares of half the size, a quorum of shares to reconstruct): every node moves 1.5 × the block per
view, so where bandwidth binds throughput doubles and the window halves with it; where latency binds the view floor is
unchanged but the window shrinks from about 150 to about 80 ms and its tenth percentile from 80 ms to a few
milliseconds, because the 67th share has to come from another region just as the 67th vote does. The right trade when
decode stays well inside 80 ms.

## What the runs showed

Base runs of 27 and 28 October 2026, 10 nodes, one 60 s step per point, against the 10-node model at the machine's
nominal bandwidth:

- Compute adds a fixed 20 ms per view the model lacks: 1 to 9 MB blocks took 22 to 34 ms per view without latency where
  the model gives 2 to 22, and the window has a 13 to 15 ms floor. Under decaf the window gains 22 to 62 ms over the
  model because the next leader's block build and encode (about 1 ms per MB) delay the proposal but not the shares; that
  compute is slack for reconstruction.
- Where bandwidth binds, the effective link is 10 to 12 Gbps, not 15: 27 to 103 MB steps on c8g.8xlarge ran 1.2 to 1.5 ×
  the model's view time, matching the ENA egress-allowance counters.
- The burstable instances (2xlarge, xlarge, large) ran at burst bandwidth, not their 0.9 to 3.75 Gbps baselines; the CPU
  bound them, never the interface.
- Every departure from the model is decode: view times exceed the model by more than 30 percent only where the median
  decode passes about 60 ms. Decode costs 0.6 ms/MB on 32 vCPU, 0.65 to 0.75 on 16, 1.05 on 8 (2.0 saturated), 1.8 to
  2.4 on 4, 3.9 to 4.2 on 2. The share of votes that waited on decode is 1 to 8 percent while the p90 decode stays
  within about 1.5 × the p10 window and 14 to 44 percent beyond it.
- The model-equivalent critical window from real timestamps lands within 20 to 40 ms of the model on 16 vCPU under decaf
  (medians 163, 156, 137, 125 ms at 9, 20, 36, 55 MB against 142, 129, 110, 87) and within 10 ms at the tenth percentile
  below 36 MB.

This is why a faster reconstruction path (the systematic concatenation, raced or waited for) bought little in those
runs: decode fits inside the window except in its tail, the systematic set cannot arrive before the first k shares of
any kind, waiting for it moved the leader's reconstruction a median 47 ms past its Cert1, and racing it spent CPU on the
machines that lacked it. The lever on the window itself is bytes per node over bandwidth.

## Running it

```
cd scripts/network-bench
uv run --with numpy python vidwindow.py point --profile decaf-2025 --nodes 100 --gbps 15 --block 50
uv run --with numpy python vidwindow.py grid --profile off --nodes 10 --threshold 2f+1 --out grid.json
uv run --with numpy python vidwindow_runs.py ../../bench-state/aws/<fleet>/runs/<run>
```

A 100-node decaf grid of 75 points takes about 15 minutes; 10 nodes run in seconds.
