//! Frame and segment-header codec for the journal storage backend, plus torn-tail scanning.
//!
//! Deliberately independent of espresso types: this module only knows about a tagged, length- and
//! crc-checked byte payload per record. `journal/state.rs` maps `Kind` to actual record types.

/// Monotonic, per-stream, strictly consecutive sequence number assigned to every record.
pub type Lsn = u64;

/// One of the two independent append-only streams a journal maintains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stream {
    Wal = 0,
    Data = 1,
}

impl Stream {
    /// Metric label value: `"wal"` or `"data"`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Wal => "wal",
            Self::Data => "data",
        }
    }
}

/// Durability class of a record: `Durable` records are acked only after an `fdatasync` covers
/// their LSN; `Enqueue` records only need to survive a process crash, not an OS crash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    Durable,
    Enqueue,
}

/// Tag identifying which `Record` variant a frame's payload decodes to. See the record-kind
/// mapping table in the design doc for the trait method that produces each one.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Snapshot = 1,
    Action = 2,
    HighQc2 = 3,
    Proposal = 4,
    Leaf = 5,
    Cert2 = 6,
    StateCert = 7,
    Upgrade = 8,
    Eqc = 9,
    NextEpochQc = 10,
    Vid = 11,
    Da = 12,
    Processed = 13,
}

impl Kind {
    fn from_u8(b: u8) -> Option<Self> {
        Some(match b {
            1 => Self::Snapshot,
            2 => Self::Action,
            3 => Self::HighQc2,
            4 => Self::Proposal,
            5 => Self::Leaf,
            6 => Self::Cert2,
            7 => Self::StateCert,
            8 => Self::Upgrade,
            9 => Self::Eqc,
            10 => Self::NextEpochQc,
            11 => Self::Vid,
            12 => Self::Da,
            13 => Self::Processed,
            _ => return None,
        })
    }
}

const SEGMENT_MAGIC: u32 = u32::from_le_bytes(*b"ESPJ");
const SEGMENT_FORMAT: u16 = 1;

/// Segment header: 40 bytes LE, crc over the first 36. Recorded once, at segment creation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentHeader {
    pub stream: Stream,
    pub seq: u64,
    pub first_lsn: Lsn,
    /// Max view written to the *previous* segment of this stream. Lets GC bound a sealed segment
    /// without scanning it: the bound for segment `seq` is `prev_max_view` of segment `seq + 1`.
    pub prev_max_view: u64,
}

impl SegmentHeader {
    pub const LEN: usize = 40;

    pub fn encode(&self) -> [u8; Self::LEN] {
        let mut buf = [0u8; Self::LEN];
        buf[0..4].copy_from_slice(&SEGMENT_MAGIC.to_le_bytes());
        buf[4..6].copy_from_slice(&SEGMENT_FORMAT.to_le_bytes());
        buf[6] = self.stream as u8;
        // buf[7] and buf[32..36] are reserved padding, left zero.
        buf[8..16].copy_from_slice(&self.seq.to_le_bytes());
        buf[16..24].copy_from_slice(&self.first_lsn.to_le_bytes());
        buf[24..32].copy_from_slice(&self.prev_max_view.to_le_bytes());
        let crc = crc32fast::hash(&buf[0..36]);
        buf[36..40].copy_from_slice(&crc.to_le_bytes());
        buf
    }

    pub fn decode(b: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(b.len() >= Self::LEN, "segment header: short read");
        let magic = u32::from_le_bytes(b[0..4].try_into().unwrap());
        anyhow::ensure!(magic == SEGMENT_MAGIC, "segment header: bad magic");
        let format = u16::from_le_bytes(b[4..6].try_into().unwrap());
        anyhow::ensure!(
            format == SEGMENT_FORMAT,
            "segment header: unsupported format {format}"
        );
        let stream = match b[6] {
            0 => Stream::Wal,
            1 => Stream::Data,
            other => anyhow::bail!("segment header: bad stream tag {other}"),
        };
        let seq = u64::from_le_bytes(b[8..16].try_into().unwrap());
        let first_lsn = u64::from_le_bytes(b[16..24].try_into().unwrap());
        let prev_max_view = u64::from_le_bytes(b[24..32].try_into().unwrap());
        let crc = u32::from_le_bytes(b[36..40].try_into().unwrap());
        anyhow::ensure!(crc32fast::hash(&b[0..36]) == crc, "segment header: bad crc");
        Ok(Self {
            stream,
            seq,
            first_lsn,
            prev_max_view,
        })
    }
}

/// Decoded record-frame header. `len` is the payload length in bytes, following this header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameHeader {
    pub len: u32,
    pub lsn: Lsn,
    pub view: u64,
    pub kind: Kind,
}

/// Record header: 32 bytes LE (`crc32(4) len(4) lsn(8) view(8) kind(1) pad(7)`), followed by
/// `len` bytes of bincode payload.
pub(crate) const FRAME_HEADER_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    Short,
    Crc,
    Lsn { expected: Lsn, got: Lsn },
}

/// Appends one frame to `out`. `view` and `kind` are stored uncompressed in the header so `scan`
/// never needs to decode `body` to bound a segment or route a record.
pub fn encode_frame(out: &mut Vec<u8>, lsn: Lsn, view: u64, kind: Kind, body: &[u8]) {
    let start = out.len();
    out.extend_from_slice(&0u32.to_le_bytes()); // crc placeholder, patched below
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&lsn.to_le_bytes());
    out.extend_from_slice(&view.to_le_bytes());
    out.push(kind as u8);
    out.extend_from_slice(&[0u8; 7]);
    out.extend_from_slice(body);
    let crc = crc32fast::hash(&out[start + 4..]);
    out[start..start + 4].copy_from_slice(&crc.to_le_bytes());
}

/// Decodes one frame at the start of `buf`. `len` only slices `buf` after the check that `buf`
/// holds that many bytes, so a corrupt length can't cause an oversized read.
///
/// Checks run cheapest-and-safest first: header present, body present, crc (which covers `lsn`),
/// then the lsn-consecutive check. A frame that fails any check ends the scan; it
/// is never partially applied.
pub fn decode_frame(
    buf: &[u8],
    expect_lsn: Lsn,
) -> Result<(FrameHeader, &[u8], usize), FrameError> {
    if buf.len() < FRAME_HEADER_LEN {
        return Err(FrameError::Short);
    }
    let len = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let total = FRAME_HEADER_LEN + len as usize;
    if buf.len() < total {
        return Err(FrameError::Short);
    }
    let crc = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    if crc32fast::hash(&buf[4..total]) != crc {
        return Err(FrameError::Crc);
    }
    let lsn = u64::from_le_bytes(buf[8..16].try_into().unwrap());
    let view = u64::from_le_bytes(buf[16..24].try_into().unwrap());
    let Some(kind) = Kind::from_u8(buf[24]) else {
        // crc passed but the kind tag is unrecognized: treat as corruption, not a new format.
        return Err(FrameError::Crc);
    };
    if lsn != expect_lsn {
        return Err(FrameError::Lsn {
            expected: expect_lsn,
            got: lsn,
        });
    }
    Ok((
        FrameHeader {
            len,
            lsn,
            view,
            kind,
        },
        &buf[FRAME_HEADER_LEN..total],
        total,
    ))
}

/// Where a scan stopped: at an exact frame boundary with no bytes left (`Clean`), or with leftover
/// bytes that don't form another valid frame (`Torn`). Both carry the byte offset of the boundary,
/// i.e. how much of `buf` to keep.
pub enum ScanEnd {
    Clean(u64),
    Torn(u64),
}

/// Scans consecutive frames starting at `first_lsn`, calling `f` on each valid one in order. Never
/// reads past the first invalid frame. Returns the scan end, the lsn the next frame must have, and
/// the max `view` seen.
pub fn scan(
    buf: &[u8],
    first_lsn: Lsn,
    mut f: impl FnMut(&FrameHeader, &[u8]) -> anyhow::Result<()>,
) -> anyhow::Result<(ScanEnd, Lsn, u64)> {
    let mut offset = 0usize;
    let mut expect_lsn = first_lsn;
    let mut max_view = 0u64;
    loop {
        if offset == buf.len() {
            return Ok((ScanEnd::Clean(offset as u64), expect_lsn, max_view));
        }
        match decode_frame(&buf[offset..], expect_lsn) {
            Ok((header, body, consumed)) => {
                f(&header, body)?;
                max_view = max_view.max(header.view);
                expect_lsn = header.lsn + 1;
                offset += consumed;
            },
            Err(_) => return Ok((ScanEnd::Torn(offset as u64), expect_lsn, max_view)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(lsn: Lsn, view: u64, kind: Kind, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        encode_frame(&mut out, lsn, view, kind, body);
        out
    }

    #[test]
    fn decode_roundtrip() {
        let buf = frame(3, 7, Kind::Action, b"hello");
        let (header, body, consumed) = decode_frame(&buf, 3).unwrap();
        assert_eq!(header.lsn, 3);
        assert_eq!(header.view, 7);
        assert_eq!(header.kind, Kind::Action);
        assert_eq!(body, b"hello");
        assert_eq!(consumed, buf.len());
    }

    #[test]
    fn scan_stops_on_lsn_gap() {
        let mut buf = frame(1, 0, Kind::Action, b"a");
        buf.extend(frame(3, 0, Kind::Action, b"b")); // lsn 2 missing
        let mut seen = vec![];
        let (end, next_lsn, _) = scan(&buf, 1, |h, b| {
            seen.push((h.lsn, b.to_vec()));
            Ok(())
        })
        .unwrap();
        assert_eq!(seen, vec![(1, b"a".to_vec())]);
        assert_eq!(next_lsn, 2);
        assert!(
            matches!(end, ScanEnd::Torn(n) if n == buf.len() as u64 - frame(3,0,Kind::Action,b"b").len() as u64)
        );
    }

    #[test]
    fn scan_clean_at_exact_end() {
        let buf = frame(1, 0, Kind::Action, b"a");
        let (end, next_lsn, _) = scan(&buf, 1, |_, _| Ok(())).unwrap();
        assert!(matches!(end, ScanEnd::Clean(n) if n == buf.len() as u64));
        assert_eq!(next_lsn, 2);
    }

    #[test]
    fn scan_torn_tail_at_every_truncation() {
        let mut full = frame(1, 0, Kind::Action, b"a");
        full.extend(frame(2, 0, Kind::Action, b"bcdef"));
        let boundary = frame(1, 0, Kind::Action, b"a").len() as u64;

        for len in 0..full.len() {
            let truncated = &full[..len];
            let mut seen = vec![];
            let (end, ..) = scan(truncated, 1, |h, b| {
                seen.push((h.lsn, b.to_vec()));
                Ok(())
            })
            .unwrap();
            if len == 0 {
                assert!(seen.is_empty());
                assert!(matches!(end, ScanEnd::Clean(0)));
            } else if (len as u64) < boundary {
                // Nothing of the first frame was ever accepted.
                assert!(seen.is_empty(), "len={len}");
                assert!(matches!(end, ScanEnd::Torn(0)), "len={len}");
            } else if (len as u64) == boundary {
                assert_eq!(seen, vec![(1, b"a".to_vec())]);
                assert!(matches!(end, ScanEnd::Clean(n) if n == boundary));
            } else {
                // Second frame is present but incomplete: first frame is still accepted.
                assert_eq!(seen, vec![(1, b"a".to_vec())]);
                assert!(matches!(end, ScanEnd::Torn(n) if n == boundary));
            }
        }
    }

    #[test]
    fn scan_rejects_every_bit_flip_in_last_frame() {
        let first = frame(1, 0, Kind::Action, b"a");
        let second = frame(2, 0, Kind::Action, b"bc");
        let boundary = first.len();

        for bit in 0..second.len() * 8 {
            let mut buf = first.clone();
            let mut corrupt = second.clone();
            corrupt[bit / 8] ^= 1 << (bit % 8);
            buf.extend(corrupt);

            let mut seen = vec![];
            let (end, ..) = scan(&buf, 1, |h, b| {
                seen.push((h.lsn, b.to_vec()));
                Ok(())
            })
            .unwrap();
            assert_eq!(seen, vec![(1, b"a".to_vec())], "bit={bit}");
            assert!(
                matches!(end, ScanEnd::Torn(n) if n == boundary as u64),
                "bit={bit}"
            );
        }
    }

    #[test]
    fn oversize_len_stops_scan_without_reading_body() {
        let mut buf = vec![0u8; FRAME_HEADER_LEN];
        buf[4..8].copy_from_slice(&(u32::MAX).to_le_bytes());
        let (end, next_lsn, max_view) = scan(&buf, 5, |_, _| Ok(())).unwrap();
        assert!(matches!(end, ScanEnd::Torn(0)));
        assert_eq!(next_lsn, 5);
        assert_eq!(max_view, 0);
    }

    #[test]
    fn segment_header_roundtrip() {
        let header = SegmentHeader {
            stream: Stream::Data,
            seq: 42,
            first_lsn: 100,
            prev_max_view: 9,
        };
        let encoded = header.encode();
        assert_eq!(encoded.len(), SegmentHeader::LEN);
        let decoded = SegmentHeader::decode(&encoded).unwrap();
        assert_eq!(decoded, header);
    }

    #[test]
    fn segment_header_rejects_bit_flip() {
        let header = SegmentHeader {
            stream: Stream::Wal,
            seq: 1,
            first_lsn: 1,
            prev_max_view: 0,
        };
        let encoded = header.encode();
        for i in 0..SegmentHeader::LEN * 8 {
            let mut corrupt = encoded;
            corrupt[i / 8] ^= 1 << (i % 8);
            if corrupt == encoded {
                continue;
            }
            assert!(SegmentHeader::decode(&corrupt).is_err(), "bit={i}");
        }
    }
}
