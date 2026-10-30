# The VID reconstruction window and the view

A model of one fast-finality view: a fluid network, the protocol's message order, and one compute cost per step where
the code spends it. It gives the view time, the window a validator has to reconstruct a block before consensus feels it,
and the time to finality, as closed forms and as a simulation, and it reads the same quantities and the compute costs
out of a run's traces. The simulation is `scripts/network-bench/vidwindow.py`, the closed forms `vidformula.py`, the run
side `vidwindow_runs.py` beside them.

## The protocol as the code runs it

Checked against `crates/hotshot/new-protocol/src` and `crates/cliquenet`; the model encodes exactly this.

- **Build.** The next leader asks for its block as soon as proposal V arrives, before its own share of V (`consensus.rs`
  `may_build_on_parked`, `request_block_and_header_if_next_leader`). The block is built from the pool and committed
  (`block.rs` `request_block`, `block_commitments`), the header follows from the parent state (`state.rs`
  `request_header`), the block is erasure-coded and unicast one fragment stream per node (`vid/disperse.rs`). Nothing
  waits for the parent payload.
- **Propose.** `maybe_propose` needs Cert1(V), the header and the block. The proposal and the Propose action are
  persisted before the broadcast (`release_proposal`); that is the `outbox wait` of the leader path.
- **Pair and vote1.** A node verifies the proposal's signatures and its share's proof (`proposal.rs`), pairs them
  (`pair_vid_share`) and starts state validation (`request_state`). `maybe_vote_1` needs the validated state and block
  V-1 reconstructed; the vote waits for the Vote action to persist (`release_vote1`), then the vote1 and the node's
  share are broadcast to every node (`SendVote1`, `BroadcastVidShare`).
- **Cert1.** Every node tallies the vote1 broadcasts; the tally sleeps until q votes are buffered, aggregates them and
  checks the aggregate once (`vote.rs` `run_tally`, `CheckedAccumulator`). The leader of V+1 proposes on its own Cert1;
  the others use theirs for vote2 and rebroadcast it.
- **Vote2 and decide.** `maybe_vote_2_and_update_lock` needs Cert1 and the reconstructed block; the vote waits for the
  lock to persist (`release_vote2`). Cert2 decides (`maybe_decide`).
- **Reconstruction** starts at the k-th share (`vid/reconstruct.rs` `try_reconstruct`), k = ceil(n / 3)
  (`init_avidm_gf2_param`); decode is parallel (`crates/vid` AVID-M GF2), `from_bytes` and the transaction commitments
  after it are not.
- **Network.** One connection per peer; a message is written to its end before the next is taken, lowest slot first
  (`cliquenet/src/net/peer.rs` `Peer::start`, `queue.rs`). A proposal or vote queued behind a share waits for it.

The earlier version of the model had the leader build at pairing rather than at the proposal's arrival, no compute, and
no vote2. It did get the message order right, which turns out to be what sets the bandwidth-bound view.

## The model

One interface per node of capacity B in each direction, flows sharing it fairly, in-order delivery per connection,
one-way delays from `latency.py`. With T the share transfer time, (n - 1) / k × block ÷ bandwidth, Q the quorum round
trip (the q-th smallest d(L, j) + d(j, L') over voters j for the leaders L of V and L' of V+1) and δ the latency left
once transfers dominate, the view is the longer of two chains (`closed_form`):

    latency chain  = Q + verify_proposal + vote1 + cert + propose + persist_proposal
    transfer chain = verify + max(vote1 + T, build + encode) + T + δ   shares and control messages in order
                   = T + decode + δ                            control messages on their own lane
    window         = max(latency chain − vote1 − T,  T + δ)   from the k-th share to the next pairing

**Two transfers a view.** Where bandwidth binds, the next leader's rebroadcast of its share of V and its dispersal of
V+1 leave its interface back to back, and its proposal of V+1 sits behind both on every connection, so no node can pair
V+1 before two share transfers have passed. The other compute hides under the transfers: build and encode finish during
the rebroadcast, decode during the next dispersal. Consecutive views alternate around the mean, because the leader of
V+1 gets proposal V late, behind L's rebroadcast, and builds late. `lane=True` models control messages passing the
shares: the dispersal then overlaps the previous view and the view is one transfer plus the decode, with every vote
waiting on the decode.

**Compute** is a `Compute` of `fixed + per_mb × block MB` costs. `vidwindow_runs.py fit` reads them from a run's traces;
this is run brendon-20261028-180812/01-volume, c8g.4xlarge (16 vCPU Neoverse-V2), decaf-2025, 9 to 92 MB blocks, medians
per load step, the preset `COMPUTE["c8g.4xlarge"]`:

| step                   | trace segment                                                | fixed ms | ms/MB |
| ---------------------- | ------------------------------------------------------------ | -------- | ----- |
| build (+commit+header) | `request_block_header_queued` → `header_created_applied`     | 0        | 1.08  |
| encode                 | `ns_disperse_start` → `ns_disperse_end`                      | 0.6      | 0.27  |
| propose                | `leaf2_commit_computed` → `proposal_signed`                  | 0.5      |       |
| persist_proposal       | `proposal_queued` → `proposal_broadcast_start`               | 7.9      |       |
| verify_proposal        | `proposal_broadcast_end` → own `proposal_validated`          | 4.2      | 0.05  |
| vote1                  | `proposal_validated` → `vote1_broadcast_start`, parent ready | 7.3      | 0.05  |
| decode (+from_bytes)   | `recover_start` → `recover_end`                              | 0        | 0.71  |
| cert                   | q-th `vote1_arrived` → `cert1_input_dispatched`              | 3.1      |       |
| vote2                  | `vote2_queued` → `vote2_broadcast_start`                     | 4.2      |       |
| decide                 | `cert2_input_dispatched` → `leaf_decided`                    | 0.6      |       |

The latency chain carries 24 ms of fixed compute a view, 12 of them storage round trips (the proposal's and the Vote
action's). The no-latency c8g.8xlarge run of the same day gives the same fixed costs within 10 percent; the steps that
parallelize cost less per MB on its 32 vCPU, encode 0.16 and decode 0.61 ms/MB.

Per leader region for 100 nodes placed as decaf-2025 places them, the f+1 threshold and no compute, the window's minimum
over bandwidth is (Q + δ) / 2, at bandwidth 2 (n - 1) / k × block ÷ (Q − δ) (`best_case`):

| leader region  | Q ms | δ ms | minimum window | at Gbps per MB of block |
| -------------- | ---- | ---- | -------------- | ----------------------- |
| eu-central-1   | 153  | 5    | 79 ms          | 0.31                    |
| ap-southeast-1 | 153  | 61   | 107 ms         | 0.51                    |
| us-east-1      | 182  | 25   | 103 ms         | 0.30                    |
| ap-southeast-2 | 157  | 53   | 105 ms         | 0.45                    |
| sa-east-1      | 312  | 128  | 220 ms         | 0.25                    |

**2f+1 threshold** (67 shares of half the size, a quorum of shares to reconstruct): every node moves 1.5 × the block per
view instead of 2.9, so where bandwidth binds throughput doubles and the window halves with it; where latency binds the
view floor is unchanged but the window shrinks from about 150 to about 80 ms and its tenth percentile from 80 ms to a
few milliseconds, because the 67th share has to come from another region just as the 67th vote does.

## The formulas

`vidformula.py` is the model without the event loop: every quantity an order statistic over the latency matrix d plus
sums of T and the compute, checked against the simulation by `tests/test_vidformula.py`. Per leader pair, L of V and L'
of V+1:

    Q          = q-th smallest over x of d(L, x) + d(x, L')                        quorum round trip
    δ_j        = d(L, L') + d(L', j) − k-th smallest over x of d(L, x) + d(x, j)   the proposal's detour
    window_j   = T + δ_j + verify_proposal                                          where transfers bind
    view       = max(Q + verify_proposal + vote1 + cert + propose + persist_proposal,
                     2T + propose + persist_proposal + d(L, L'))
    finality   = q-th_i [ q-th_j [ cert1_j + vote2 + d(j, i) ] + cert + decide ]
    cert1_j    = q-th_x [ d(L, x) + verify_proposal + vote1 + d(x, j) ] + cert

δ_j is the window's floor: in the transfer-bound simulation no window is below T + δ_j, three quarters sit on it to the
millisecond and the rest above it by whole cross-region hops, when the proposal of V+1 crossed behind more of the
leader's queue. What the earlier model read from the simulation is now read off the matrix. The pair formulas hold for
the mean, since consecutive views alternate around it; `run_views` evaluates a leader sequence as a recurrence, each
view's timestamps for every node from the previous view's, and `point` reports the same figures as the simulation's. Its
one approximation is that every flow of a transfer finishes together, T after it starts, where the fluid simulation lets
flows to idle receivers finish first; so where bandwidth binds its k-th shares arrive later and its windows and finality
come out longer, by up to a fifth, while the view's mean agrees within 4 percent:

| setting (10 nodes, c8g.4xlarge compute) | view mean sim / formula | finality p50 | window p50 |
| --------------------------------------- | ----------------------- | ------------ | ---------- |
| decaf, 1 MB, 100 Gbps                   | 174.7 / 174.7           | 261 / 261    | 170 / 160  |
| decaf, 49 MB, 15 Gbps                   | 220.4 / 223.2           | 322 / 365    | 111 / 111  |
| no latency, 92 MB, 15 Gbps              | 227.8 / 236.3           | 395 / 412    | 102 / 115  |
| no latency, 92 MB, 15 Gbps, lane        | 178.2 / 178.3           | 352 / 352    | −57 / −52  |

Against the decaf run at 11 Gbps (`vidwindow_runs.py --model formula --gbps 11`) the formula's view is within 8 percent
and finality within 12 percent at every step, in under a second:

| MB/s | block MB | view run / formula | finality run / formula |
| ---- | -------- | ------------------ | ---------------------- |
| 50   | 9        | 181 / 182          | 272 / 273              |
| 100  | 19       | 185 / 195          | 283 / 298              |
| 150  | 33       | 213 / 220          | 328 / 337              |
| 200  | 49       | 242 / 245          | 384 / 401              |
| 250  | 82       | 330 / 356          | 493 / 554              |
| 300  | 92       | 397 / 390          | 562 / 614              |

`vidformula.py pairs` tabulates the closed forms per region pair; for 100 nodes placed as decaf-2025 at 15 Gbps and 50
MB blocks it takes a second. Where the two leaders share a region the latency chain binds at 185 ms (eu-central-1,
ap-southeast-1) to 340 ms (sa-east-1); across regions the transfer chain adds d(L, L') to the two transfers and binds
instead, 243 ms between eu-central-1 and ap-southeast-1 and 331 ms between ap-southeast-1 and sa-east-1. Finality from
an eu-central-1 leader is 266 ms, from sa-east-1 351 ms.

## What the runs showed

`vidwindow_runs.py` on the decaf-2025 run above, the model at the fitted compute (view and finality in ms, `late` the
share of vote1s whose parent block was reconstructed after the proposal paired):

| MB/s | block MB | view run / model 15 / model 11 Gbps | finality run / model 15 / 11 | proposal transit run / model | late run / model |
| ---- | -------- | ----------------------------------- | ---------------------------- | ---------------------------- | ---------------- |
| 50   | 9        | 181 / 182 / 182                     | 272 / 266 / 273              | 91 / 51                      | 1% / 2%          |
| 100  | 19       | 185 / 183 / 191                     | 283 / 286 / 302              | 62 / 71                      | 4% / 6%          |
| 150  | 33       | 213 / 220 / 220                     | 328 / 301 / 309              | 92 / 79                      | 8% / 6%          |
| 200  | 49       | 242 / 222 / 232                     | 384 / 323 / 347              | 109 / 79                     | 14% / 7%         |
| 250  | 82       | 330 / 260 / 329                     | 493 / 421 / 527              | 138 / 91                     | 20% / 8%         |
| 300  | 92       | 397 / 286 / 360                     | 562 / 466 / 592              | 161 / 106                    | 17% / 12%        |

- With the compute in, the view matches within 5 percent up to 33 MB blocks and finality within 10 percent. The fixed 20
  ms a view the compute-free model lacked is the table above.
- Above 50 MB the run is slower than the model at the nominal 15 Gbps and matches it at 11 Gbps, the effective link the
  ENA egress-allowance counters showed before. Bandwidth is the knob to set from the run, not a model term.
- The proposal's transit to the next leader (`proposal_queued` to its `request_block_header_queued`) is 60 to 160 ms
  here and 13 to 100 ms on the no-latency run, for a message of a few kilobytes: the in-order connection, as modelled.
- The run's late votes exceed the model's because its decode has a tail the model lacks (p90 decode is 1.4 × p50 at 9 MB
  and 2.2 × at 92 MB). The model's `late` is the floor; the gap to it is the tail.
- Halving the decode alone changes nothing today (the view stays at two transfers); it is worth 20 percent more once a
  lane makes the decode the bound.

## Levers the model ranks

For 10 nodes at 15 Gbps and the c8g.4xlarge compute, mean view time in ms (`vidwindow.py point`):

| block MB | in order (today) | control lane | lane, decode halved |
| -------- | ---------------- | ------------ | ------------------- |
| 20       | 61               | 47           | 44                  |
| 49       | 126              | 95           | 77                  |
| 92       | 229              | 178          | 144                 |

- **A lane for control messages** (a second connection per peer, or interleaving a small message's frames into a
  share's) takes the bandwidth-bound view from two transfers to one plus the decode: 1.3 × the throughput at 92 MB, and
  the decode becomes the next bound. Under decaf-2025 latency at these sizes the view is latency-bound and the lane
  changes nothing.
- **Decode** then sets the view directly, so faster recovery or the 2f+1 threshold pays where it did not before.
- **The latency chain** has 24 ms of compute, half of it two storage round trips; batching the Vote action write with
  the proposal's, or acknowledging before fsync, is worth up to 12 ms a view where latency binds.

## Blocks that follow a rate

A run offers a rate, not a block size: the leader takes what is in its pool, so the block is the rate times the view,
and a slower view makes a bigger block. `vidformula.Load` closes that loop, `sweep` evaluates a list of offered rates
with blocks following the rate under a maximum block, and `vidwindow_runs.py --closed-loop` compares a run step by step
at its offered rate rather than at its measured block. **Capacity** in the explorer and in `capacity` is the peak of
throughput over block size under the cap: the rate at which the view, growing with the block, stops delivering more
bytes per second. It is a steady-state figure, and the runs below show the network tips before it.

## The October 2026 runs

Fleets of 5 to 20 nodes under `bench-state/aws/brendon-202610{29,30}-*`, the new protocol with f+1 reconstruction
(`ESPRESSO_BENCH_VID_TWO_THIRDS` for 2f+1), 400 MB blocks unless a cap is named, the single-region topology (`off`) and
decaf-2025 latency. Fits and comparisons are `vidwindow_runs.py fit` and `--closed-loop`.

**2f+1 against f+1** (10 × c8g.8xlarge, eu-west-1). At 15 Gbps the share transfer T falls from 1.2 to 0.69 ms per MB of
block, but the next leader's build and encode, 1.36 ms/MB on this instance, then outlast the rebroadcast they hid under:
the view is verify + max(vote1 + T, build + encode) + T, and the compute term binds. Measured capacity: f+1 300 to 332
MB/s, 2f+1 370 ± 10; decaf f+1 230 ± 10, 2f+1 300 ± 10. The model says taking build and encode off the critical path,
not the decode, is what releases the other half of the 2f+1 gain.

**The matrix** (us-east-1, 12 fleets, c8g.2xlarge to c8g.16xlarge, 5, 10 and 20 nodes, about 100 USD). Measured capacity
in MB/s, the model's capacity in brackets at the run's fitted compute and the instance's nominal bandwidth (15 Gbps, 30
for the 16xlarge):

| nodes, latency | c8g.2xlarge | c8g.4xlarge | c8g.8xlarge | c8g.16xlarge |
| -------------- | ----------- | ----------- | ----------- | ------------ |
| 5, off         | 250 (495)   | 295 (388)   | 260 (427)   | 510 (449)    |
| 10, off        | 220 (342)   | 300 (388)   | 285 (394)   | 415 (672)    |
| 20, off        | 220 (235)   | 285 (324)   | 245 (327)   | 345 (596)    |
| 5, decaf       | 210 (293)   | 230 (360)   | 260 (394)   | 320 (582)    |
| 10, decaf      | 230 (282)   | 170 (371)   | 250 (375)   | 270 (614)    |
| 20, decaf      | 190 (256)   | 220 (316)   | 205 (316)   | 300 (510)    |

Below capacity the model holds: per-step medians of view and finality are within 2 to 9 percent on decaf and 10 to 30
percent on the single-region topology, where a 20 ms view leaves no room for the few milliseconds the model lacks.
Capacity itself is overpredicted everywhere but the 5-node 16xlarge cell, because the runs do not reach the steady state
the model computes. The compute per instance type fitted from these runs, in ms per MB of block:

| instance     | build + encode | decode |
| ------------ | -------------- | ------ |
| c8g.2xlarge  | 2.00           | 1.18   |
| c8g.4xlarge  | 1.43           | 0.69   |
| c8g.8xlarge  | 1.04           | 0.51   |
| c8g.16xlarge | 0.91           | 0.47   |

The fixed costs, 7 ms each for the proposal's persistence and the vote, are the same on every type.

**Why a run tips.** Above capacity a slow view leaves more transactions in the next leader's pool, the bigger block
makes a longer view, and the loop runs to the 400 MB cap in a few views, with 100 to 400 MB sweeps, the builder's 500 ms
`empty_block_delay` once the pool runs dry, and the submitter's 5 s in-flight window starving. The trigger is not CPU,
NIC, disk or retries (at most 22 transactions were outstanding at the tipping point on the 16xlarge). It is the submit
path: transactions are forwarded as 1 MB messages to the leaders of views v+2 and v+3 (`block.rs` `SEND_LEAD`, fanout 2)
over the same in-order cliquenet connections that carry proposals and votes, and the coordinator hashes every forward
(0.56 ms each). Proposal arrival by receiver role shows it, at 390 MB/s on 10 × c8g.16xlarge: 24 to 26 ms at the leaders
of v+1 to v+3 against 19 ms elsewhere and 11 ms at 100 MB/s. Control messages into the upcoming leaders queue behind
forwards, which is the same mechanism the model has for shares, one hop earlier.

**The maximum block** (10 × c8g.8xlarge, off, 100 and 150 MB caps against 400). The model's capacity barely moves with
the cap, 364 MB/s at 50 MB to 379 at 400, while finality at capacity goes from 224 ms to 1.8 s. The runs: at 100 MB the
oscillation is gone, past capacity the network keeps deciding 261 to 275 MB/s with no empty views, blocks pinned at 99
MB and 335 ms views, where the 400 MB cap decided 155 to 207. At 150 MB the oscillation returns, 200 to 226 MB/s
decided, 2 to 20 percent empty views, bimodal view times (p50 34 ms, p90 315). Decaf at 150 MB decided 242 to 262 but
with p50 latency of 1.5 to 4 s. The rule that fits: keep the view at the cap under a third of the submitter's in-flight
window, which puts the cap near 100 MB for this hardware, 200 MB if a 1 s finality bound is the only constraint.

**Where the model is short at large blocks.** The real 99 MB view is 318 to 335 ms against the model's 264. Two costs
the traces do not isolate grow with the block: the next leader verifies its own share before voting, about 0.3 ms/MB at
f+1, so its vote1 leaves 35 ms later and the dispersal falls behind the rebroadcast instead of beside it (+26 ms); and
the proposal crosses about 33 ms after the dispersal's nominal end, a per-connection drain tail of about a quarter of T
that the fluid model's simultaneous finish does not have. Both scale with block size, so the model is right at small
blocks and under by a quarter at the cap, and its capacities above are optimistic by about that much.

**Intel against Graviton** (10 × c8i.8xlarge against c8g.8xlarge, 100 MB cap). Capacity is the same place, 325 ± 25
against 300 single-region and 250 ± 25 against 230 to 250 on decaf, because at these rates the view is mostly transfer;
the compute difference shows as larger blocks for the same throughput. Per MB of block at matched sizes, Granite Rapids
decodes 1.9 to 2.0 × slower (1.0 to 1.16 ms/MB against 0.54 to 0.58), builds 1.2 to 1.3 × slower, encodes 1.4 × slower,
and its proposal verification grows with the block where Graviton's stays at 5 to 6 ms. The ratios being the same at
every block size points at the erasure-code kernel rather than memory or hashing.

## Next steps

Model:

- **Own-share verification** as a traced step and a per-MB cost on the pairing path, and a **per-connection tail** on
  the proposal's crossing, the two quarter-of-T terms above; then re-fit the matrix and the capacity table should close.
- **An ingestion ceiling**: the forward path (two 1 MB forwards per transaction, hashed on the coordinator) as a term
  that grows with rate, so the model predicts the tipping point, not only the steady state it never reaches.
- **The recurrence's finality** is conservative against the fluid simulation by up to a third at large blocks (one
  transfer time for every flow); worth either accepting in the doc or giving the recurrence the fluid finish.
- **Rate-mode tests** for `sweep` and `capacity` against the matrix's per-step tables, so the closed loop stays checked.

Protocol, in the order the model ranks them:

- **A control lane**, or forwards and shares off the in-order connections, or forward hashing off the coordinator:
  removes the submit-path queueing that tips the runs and is the lane the model already values at 1.3 × throughput.
- **Build and encode off the proposal's critical path** (encode in the builder task, or propose before the dispersal
  ends): the other half of the 2f+1 gain.
- **A 100 MB default maximum block** on this hardware, revisited with the lane in.
- **A `recover` microbenchmark** on c8i and c8g to confirm the erasure kernel is the Intel gap; minutes, no fleet.

Measurement:

- Decaf at the 100 MB cap, and runs with a higher driver in-flight cap (`--cap-s`) to measure the plateau past capacity
  rather than the tipping point.
- The matrix scripts (`cell.sh`, `analyze.py`, `summary.py`) live outside the repo; they belong under
  `scripts/network-bench` with the harness so the table can be rerun on a new image.

## Running it

```
cd scripts/network-bench
uv run --with numpy python vidwindow.py point --profile decaf-2025 --nodes 100 --gbps 15 --block 50
uv run --with numpy python vidwindow.py point --profile off --nodes 10 --gbps 15 --block 50 --compute c8g.4xlarge --lane
uv run --with numpy python vidwindow.py grid --profile off --nodes 10 --threshold 2f+1 --out grid.json
uv run --with numpy python vidwindow_runs.py fit ../../bench-state/aws/<fleet>/runs/<run>
uv run --with numpy python vidwindow_runs.py ../../bench-state/aws/<fleet>/runs/<run> [--gbps 11] [--compute off] [--model formula]
uv run --with numpy python vidformula.py point --profile off --nodes 10 --gbps 15 --block 92 --compute c8g.4xlarge --check
uv run --with numpy python vidformula.py pairs --profile decaf-2025 --nodes 100 --gbps 15 --block 50 --compute c8g.4xlarge
uv run --with numpy python vidformula.py sweep --profile off --nodes 10 --gbps 15 --compute c8g.8xlarge --max-block 100 --rates 100,200,300,350
uv run --with numpy python vidwindow_runs.py ../../bench-state/aws/<fleet>/runs/<run> --closed-loop
```

A 100-node decaf grid of 75 points takes about 15 minutes; 10 nodes run in seconds. `vidformula-explorer.html` is the
closed forms in a browser, with a rate mode and the capacity tile; open it from a checkout, it has no dependencies.
