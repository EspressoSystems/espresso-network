# AvidM Encoding & Decoding Algorithm

## 1. Introduction

**AvidM** (Asynchronous Verifiable Information Dispersal with Merkle trees) is an erasure-coding-based VID scheme derived from the [DispersedLedger paper (NSDI'22)](https://www.usenix.org/conference/nsdi22/presentation/yang).

A consensus leader disperses a block payload to validators so that any subset holding sufficient cumulative weight can recover the original data. Each validator receives a compact share it can independently verify against a single commitment (a Merkle root).

## 2. Parameters

| Symbol | Meaning | Source |
|--------|---------|--------|
| `n` | `total_weights` — sum of all validator weights; equals the number of Reed-Solomon codeword symbols | `AvidMParam` |
| `k` | `recovery_threshold` — minimum cumulative weight needed to recover the payload; equals the RS polynomial degree bound | `AvidMParam` |
| `distribution[i]` | Weight of validator `i`; determines how many contiguous codeword indices it receives | `disperse()` arg |
| Field | BN254 scalar field (`ark_bn254::Fr`), a 254-bit prime field with 31 usable bytes per element | `config.rs` |

Constraint: `0 < k <= n`, and `n` must be a power of two or the next power of two is used for the FFT domain.

## 3. Encoding (Dispersal)

Encoding proceeds in four steps, implemented across `pad_to_fields`, `raw_encode`, and `distribute_shares`.

### Step 1: Payload to field elements (`pad_to_fields`)

1. Compute `elem_bytes_len = (MODULUS_BIT_SIZE - 1) / 8` — for BN254 this is **31 bytes** per field element.
2. Compute `num_bytes_per_chunk = k * elem_bytes_len`.
3. Append a `0x01` byte to the payload, then zero-pad until the total length is a multiple of `num_bytes_per_chunk`.
4. Convert the padded byte stream into field elements: each group of `elem_bytes_len` bytes becomes one element via `F::from_le_bytes_mod_order` (little-endian).

Result: a vector of field elements whose length is divisible by `k`.

### Step 2: Reed-Solomon encoding (`raw_encode`)

1. Create an FFT domain of size `n` using radix-2 roots of unity (`ω^0, ω^1, ..., ω^{n-1}`).
2. Partition the field elements from Step 1 into chunks of `k` elements each. Call the number of chunks `c`.
3. For each chunk, treat the `k` elements as coefficients of a degree `k-1` polynomial and FFT-evaluate it at all `n` domain points. Truncate the result to exactly `n` evaluations.

Result: `c` codewords, each of length `n`.

### Step 3: Merkle commitment

1. **Transpose** the codeword matrix: `raw_share[j] = [codeword_0[j], codeword_1[j], ..., codeword_{c-1}[j]]` for `j` in `0..n`. Each raw share is a column vector of `c` field elements.
2. **Hash** each raw share: `leaf[j] = Hash(serialize(raw_share[j]))` where `Hash` is the configured hash function (Keccak256 by default; SHA256 or Poseidon2 via feature flags).
3. **Build** a Merkle tree over the `n` leaf digests.
4. **Commitment** = Merkle root (32 bytes).

### Step 4: Distribution (`distribute_shares`)

1. Assign contiguous index ranges per validator using a prefix-sum over weights:
   - Validator `i` gets indices `[sum_{j<i} w_j, sum_{j<=i} w_j)`.
2. Each validator `i` receives an `AvidMShare` containing:
   - `index`: the validator's ordinal index
   - `payload_byte_len`: the original payload length in bytes (needed for unpadding)
   - `range`: the contiguous index range assigned
   - `payload`: the raw shares (column vectors) for those indices
   - `mt_proofs`: one Merkle membership proof per index

## 4. Verification (`verify_share`)

To verify a share against a commitment:

1. Check that the share's `range` does not exceed `n` and that the number of payload entries and proofs matches `range.len()`.
2. For each index `j` in the share's range:
   - Recompute `leaf = Hash(serialize(raw_share[j]))`.
   - Verify the Merkle proof against the commitment at position `j`.
3. Accept only if **all** proofs pass.

## 5. Recovery (`recover`)

Given a set of shares whose cumulative weight is at least `k`:

1. **Collect** evaluation points: iterate through shares, extracting `(index, raw_share)` pairs. Stop once `k` points are gathered (additional shares are ignored).
2. **Determine** `num_polys` (= `c`, the number of chunks) from the length of any raw share vector.
3. **Recover** all chunk polynomials using the **erasure locator polynomial** method described below.
4. **Flatten** all recovered coefficients back to bytes via `field_to_bytes` (each field element → `elem_bytes_len` little-endian bytes).
5. **Strip padding**: scan from the end of the byte vector to find the last `0x01` byte, truncate everything from that byte onward.

### Erasure Locator Polynomial Recovery

Let `N` = `domain.size()` (the FFT domain size, a power of two ≥ `n`), `ω` = primitive `N`-th root of unity. The encoding evaluates each chunk polynomial at `ω^0, ..., ω^{n-1}` (using only the first `n` points of the size-`N` domain).

Let `R` = set of received evaluation indices (|R| ≥ `k`), and `Ω = {0..N} \ R` = all erased indices (includes both missing shares within `0..n` and the unused domain points `n..N`).

**One-time setup** (shared across all `c` polynomials):

Since `x^N - 1 = ∏_{j=0}^{N-1}(x - ω^j) = E(x) · R(x)`, where `R(x) = ∏_{j ∈ R}(x - ω^j)` is the **received locator** and `E(x) = ∏_{j ∈ Ω}(x - ω^j)` is the erasure locator, we compute `E`'s evaluations directly from the derivative of `R` at received points.

Differentiating: `N·x^{N-1} = E'(x)·R(x) + E(x)·R'(x)`. At received points `ω^j` (where `R(ω^j) = 0`): `E(ω^j) = N·ω^{-j} / R'(ω^j)`, where `R'(ω^j) = ∏_{i∈R, i≠j}(ω^j - ω^i)` is computable directly from received indices.

1. Compute **E's evaluations** at all `N` domain points — O(k²):
   - For `j ∈ R`: `E(ω^j) = N · ω^{-j} / R'(ω^j)` via the product formula and batch inversion.
   - For `j ∈ Ω`: `E(ω^j) = 0` by definition.
2. **IFFT** the `N` evaluations of `E` → coefficient form `E_coeffs` — O(N log N).
3. Compute `E'(x)` via **formal derivative** of `E_coeffs`, then **FFT** → evaluations `E'(ω^j)` — O(N log N).
4. For `j ∈ Ω`: compute `1/E'(ω^j)` via **batch inversion** of `E'` at erased points.

**Per-polynomial recovery** (for each chunk polynomial `C_p`, `p = 0..c`, parallelized):

1. Construct evaluations of the product `P(x) = C_p(x)·E(x)`:
   - For `j ∈ R`: `P(ω^j) = C_p(ω^j) · E(ω^j)` (both values known).
   - For `j ∈ Ω`: `P(ω^j) = 0` (since `E` vanishes on `Ω`).
   All `N` evaluations of `P` are known.
2. IFFT `P` → coefficient form. `P` has degree ≤ `(k-1) + |Ω| ≤ N-1`.
3. Compute formal derivative `P'(x)` in coefficient form.
4. FFT `P'` → evaluations at all domain points.
5. Recover erased values using the **product rule identity**: at each `j ∈ Ω`,
   `C_p(ω^j) = P'(ω^j) / E'(ω^j)`.
   (This follows from the Leibniz rule: since `E(ω^j) = 0`, `(C·E)'(ω^j) = C(ω^j)·E'(ω^j)`.)
6. Combine received and recovered evaluations, IFFT → take first `k` coefficients.

**Complexity**:
- One-time: O(k²) for computing E's evaluations at received points, O(N log N) for IFFT of E and FFT of E'.
- Per polynomial: 2 IFFTs + 1 FFT + O(N) pointwise operations = O(N log N).
- **Total: O(c · N log N) + O(k²)**, vs the previous O((N-k)²) erasure locator approach.

## 6. Worked Example

Parameters: `k = 2`, `n = 4`, `distribution = [1, 1, 1, 1]`, `payload = [0xAB]`.

**Step 1 — Padding:**
- `elem_bytes_len = 31`, `num_bytes_per_chunk = 2 * 31 = 62`
- Padded bytes: `[0xAB, 0x01, 0x00, ..., 0x00]` (62 bytes total)
- Field elements: `[F(0xAB + 0x01*256), F(0)]` = 2 elements (one chunk of `k = 2`)

**Step 2 — RS encoding:**
- One chunk `[a_0, a_1]` represents polynomial `p(x) = a_0 + a_1 * x`
- FFT evaluates at `ω^0, ω^1, ω^2, ω^3` → 4 codeword symbols

**Step 3 — Merkle tree:**
- Each raw share is a single field element (only 1 chunk): `raw_share[j] = [codeword[j]]`
- 4 leaves → Merkle tree with root = commitment

**Step 4 — Distribution:**
- Validator 0 gets index 0, Validator 1 gets index 1, etc.
- Each receives 1 raw share + 1 Merkle proof

**Recovery (from any 2 shares):**
- E.g., shares at indices 0 and 2 → evaluation points `(ω^0, p(ω^0))` and `(ω^2, p(ω^2))`
- Lagrange interpolation recovers `[a_0, a_1]`
- Convert back to bytes, strip padding → `[0xAB]`

## 7. Hash Configurations

The hash function is selected at compile time via feature flags. All configurations implement the `AvidMConfig` trait:

| Config | Feature Flag | `BaseField` | Hash | Merkle Tree |
|--------|-------------|-------------|------|-------------|
| `Keccak256Config` | `keccak256` | `ark_bn254::Fr` | Keccak256 | `HasherMerkleTree<Keccak256>` |
| `Sha256Config` | `sha256` | `ark_bn254::Fr` | SHA256 | `HasherMerkleTree<Sha256>` |
| `Poseidon2Config` | (default) | `ark_bn254::Fr` | Poseidon2 | Poseidon2-based tree |

The `raw_share_digest` method serializes field elements with `serialize_uncompressed` and feeds the bytes into the hasher.

## 8. GF2 Variant (`AvidmGf2Scheme`)

`AvidmGf2Scheme` (in `avidm_gf2.rs`) is an alternative implementation using a binary field (GF(2)) instead of BN254. Key differences:

| Aspect | AvidM (BN254) | AvidM-GF2 |
|--------|--------------|-----------|
| **Field** | BN254 scalar field (254-bit prime) | GF(2) binary field |
| **RS library** | `ark_poly` FFT | `reed_solomon_simd` (byte-level) |
| **Byte→field conversion** | 31 bytes → 1 field element | None needed; operates directly on bytes |
| **Padding** | `0x01` + zeros to fill `k * 31`-byte chunks | `0x01` + zeros; shard size must be even |
| **Share payload type** | `Vec<Vec<F>>` (field elements) | `Vec<Vec<u8>>` (raw bytes) |
| **Hash** | Configurable (Keccak256/SHA256/Poseidon2) | Keccak256 only |
| **Merkle commitment** | Same structure | Same structure |
| **Protocol version** | V0.3–V0.5 | V0.6+ (`Vid2UpgradeVersion`) |

The encoding layout differs: GF2 splits the padded payload into `k` original shards, then produces `n - k` recovery shards via `reed_solomon_simd::encode`. The first `k` shares in the Merkle tree are original data; the remaining `n - k` are parity.

## 9. Properties

| Property | Value |
|----------|-------|
| Polynomial degree | `k - 1` |
| Evaluation points | `n` roots of unity in BN254 (or `n` GF2 indices) |
| Commitment size | 32 bytes (single Merkle root hash) |
| Recovery threshold | Any `k` shares by cumulative weight |
| Padding scheme | `0x01` byte marker + trailing zeros |
| Bytes per field element | 31 (BN254) |
| Share size per validator | `weight_i * c` field elements + `weight_i` Merkle proofs |

## 10. Benchmark Results: FFT vs Lagrange Recovery

Benchmarks comparing the FFT-based erasure locator recovery (Section 5) against the original per-polynomial Lagrange interpolation approach. Parameters: `(k=34, n=100)` with uniform weight-1 distribution, recovering from `k` shares.

**Machine:** Apple Silicon (macOS), Rust 1.x release profile, `--features keccak256`.

| Payload | FFT Recovery | Lagrange Recovery | Speedup |
|---------|-------------|-------------------|---------|
| 1 MB (~1065 polynomials) | 12.08 ms | 441.20 ms | **36.5x** |
| 5 MB (~5325 polynomials) | 57.91 ms | 2229.1 ms | **38.5x** |

The FFT approach amortizes the O(k²) erasure locator setup across all polynomials, then does O(N log N) per polynomial. The Lagrange approach pays O(k²) independently for each polynomial. As the number of polynomials (`c`) grows with payload size, the FFT advantage increases.

For reference, dispersal times: 7.83 ms (1 MB), 35.61 ms (5 MB). Verification: 70.7 µs (1 MB), 346.5 µs (5 MB).

## Source Files

- `vid/src/avidm.rs` — Core algorithm: pad, encode, distribute, verify, recover
- `vid/src/avidm/config.rs` — Hash configuration trait and implementations
- `vid/src/utils/bytes_to_field.rs` — Bidirectional field element ↔ bytes conversion
- `vid/src/lib.rs` — `VidScheme` trait definition
- `vid/src/avidm_gf2.rs` — GF2 variant for comparison
