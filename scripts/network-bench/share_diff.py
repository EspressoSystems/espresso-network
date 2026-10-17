#!/usr/bin/env python3
"""Compares two share lanes of a node, before and after a share heal.

Usage: share_diff.py <orig_dir> <healed_dir>

Reads the `*.log` segments of each directory (layout of `crates/journal-lane/src/format.rs`),
keeps the body of the last Vid frame (kind 11) per key and prints
`equal=N differ=N orig_only=N healed_only=N`. Exit 1 if any key differs.
"""

import struct
import sys
import zlib
from pathlib import Path

SEGMENT_MAGIC = b"ESPJ"
SEGMENT_HEADER_LEN = 40
FRAME_HEADER_LEN = 32
KIND_VID = 11

# crc32(4) len(4) lsn(8) key(8) kind(1) stamp(4) pad(3)
_FRAME = struct.Struct("<IIQQBI3x")


def vid_bodies(lane_dir: Path) -> dict[int, bytes]:
    """Body of the last Vid frame per key over the segments of `lane_dir` in name order.
    A torn tail ends a segment like the node's scan does. Raises on a bad segment header."""
    bodies: dict[int, bytes] = {}
    for path in sorted(lane_dir.glob("*.log")):
        for key, kind, body in _frames(path.read_bytes(), path):
            if kind == KIND_VID:
                bodies[key] = body
    return bodies


def _frames(buf: bytes, path: Path):
    if len(buf) < SEGMENT_HEADER_LEN or buf[:4] != SEGMENT_MAGIC:
        raise ValueError(f"{path}: bad segment header")
    (crc,) = struct.unpack_from("<I", buf, 36)
    if zlib.crc32(buf[:36]) != crc:
        raise ValueError(f"{path}: bad segment header crc")
    (expect_lsn,) = struct.unpack_from("<Q", buf, 16)
    offset = SEGMENT_HEADER_LEN
    while len(buf) - offset >= FRAME_HEADER_LEN:
        crc, length, lsn, key, kind, _stamp = _FRAME.unpack_from(buf, offset)
        end = offset + FRAME_HEADER_LEN + length
        if end > len(buf) or zlib.crc32(buf[offset + 4 : end]) != crc:
            return
        if lsn != expect_lsn:
            return
        yield key, kind, buf[offset + FRAME_HEADER_LEN : end]
        expect_lsn += 1
        offset = end


def diff(orig: dict[int, bytes], healed: dict[int, bytes]) -> dict[str, int]:
    common = orig.keys() & healed.keys()
    equal = sum(orig[k] == healed[k] for k in common)
    return {
        "equal": equal,
        "differ": len(common) - equal,
        "orig_only": len(orig.keys() - healed.keys()),
        "healed_only": len(healed.keys() - orig.keys()),
    }


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    counts = diff(vid_bodies(Path(argv[0])), vid_bodies(Path(argv[1])))
    print(" ".join(f"{name}={n}" for name, n in counts.items()))
    return 1 if counts["differ"] else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
