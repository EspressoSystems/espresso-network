# Instance types: leader block build

Measured 2026-10-05, eu-west-1, Ubuntu 24.04, on-demand prices.

- Bench: `crates/espresso/types/benches/block_build.rs` on `ma/payload-copies` (2a8d8f39970), criterion, built by GitHub
  Actions on `ubuntu-24.04` / `ubuntu-24.04-arm` (release profile, no `target-cpu`).
- Metric: `request_block` (payload build, encode, VID commitment and builder commitment in parallel), 10 nodes, 1 MB
  transactions over 16 namespaces unless noted, default glibc malloc, mean ms.
- arm64 numbers include `sha2` feature `asm` on aarch64. Without it `sha2` 0.10 hashes in software and Graviton
  `request_block` is 3.6-4x slower (c8g 33 MB: 90.8 ms).
- `GLIBC_TUNABLES=glibc.malloc.hugetlb=1` changed no 33 MB result by more than 1 ms.
- Relative columns: time vs the fastest type, cost (USD/h x 33 MB time) vs the cheapest; lower is better.

| Type          | CPU                   | vCPU | USD/h | 5 MB | 10 MB | 20 MB | 33 MB | 33 MB, 100 KB tx | 50 MB | 33 MB time | cost/block |
| ------------- | --------------------- | ---: | ----: | ---: | ----: | ----: | ----: | ---------------: | ----: | ---------: | ---------: |
| m8azn.6xlarge | AMD EPYC Turin 5 GHz  |   24 |  2.76 | 2.24 |  4.46 |  8.96 |  14.9 |             15.7 |  30.3 |       1.00 |       2.65 |
| m8azn.3xlarge | AMD EPYC Turin 5 GHz  |   12 |  1.38 | 2.25 |  4.57 |  9.04 |  15.3 |             15.2 |  29.5 |       1.03 |       1.36 |
| c8a.4xlarge   | AMD EPYC Turin        |   16 |  0.93 | 2.54 |  4.94 |  10.3 |  17.3 |             17.4 |  33.3 |       1.17 |       1.03 |
| m9g.4xlarge   | Graviton5             |   16 |  0.87 | 2.96 |  5.92 |  11.9 |  19.6 |             20.0 |  36.6 |       1.32 |       1.10 |
| c7a.4xlarge   | AMD EPYC Genoa        |   16 |  0.88 | 4.62 |  8.75 |  12.9 |  21.4 |             26.2 |  58.3 |       1.44 |       1.21 |
| c8g.4xlarge   | Graviton4             |   16 |  0.68 | 3.51 |  7.17 |  14.1 |  23.4 |             23.7 |  44.1 |       1.57 |       1.03 |
| c7g.4xlarge   | Graviton3             |   16 |  0.62 | 3.81 |  7.62 |  15.2 |  25.0 |             25.1 |  51.8 |       1.68 |       1.00 |
| c8i.4xlarge   | Intel Granite Rapids  |   16 |  0.80 | 3.93 |  7.28 |  14.0 |  25.9 |             25.8 |  51.9 |       1.74 |       1.34 |
| c7i.4xlarge   | Intel Sapphire Rapids |   16 |  0.77 | 4.53 |  8.69 |  17.6 |  31.5 |             31.4 |  60.2 |       2.12 |       1.56 |

- m8azn.6xlarge is 3% faster than m8azn.3xlarge: the build scales with per-core speed, not core count.
- c8a: 17% slower than the fastest type at 3% above the cheapest cost per block.
- Intel types are slower and cost more per block than c8a and every Graviton type.
