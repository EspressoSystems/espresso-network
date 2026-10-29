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
    transfer chain = 2 T + propose + persist_proposal + δ      shares and control messages in order
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
```

A 100-node decaf grid of 75 points takes about 15 minutes; 10 nodes run in seconds.
