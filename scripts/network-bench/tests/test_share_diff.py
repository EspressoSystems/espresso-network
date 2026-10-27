import struct
import zlib
from pathlib import Path

import pytest
import share_diff as sd

VID = 11
OTHER = 2


def segment(frames: list[tuple[int, int, bytes]], first_lsn: int = 0) -> bytes:
    head = struct.pack("<4sHBxQQQI", b"ESPJ", 1, 3, 0, first_lsn, 0, 0)
    out = head + struct.pack("<I", zlib.crc32(head))
    for i, (key, kind, body) in enumerate(frames):
        rest = struct.pack("<IQQBI3x", len(body), first_lsn + i, key, kind, 7) + body
        out += struct.pack("<I", zlib.crc32(rest)) + rest
    return out


def lane(path: Path, *segments: bytes) -> Path:
    path.mkdir()
    for i, seg in enumerate(segments):
        (path / f"{i:016x}.log").write_bytes(seg)
    return path


def test_layout_matches_rust():
    assert len(segment([])) == sd.SEGMENT_HEADER_LEN
    assert len(segment([(1, VID, b"")])) == sd.SEGMENT_HEADER_LEN + sd.FRAME_HEADER_LEN


def test_vid_bodies_last_frame_per_key_and_vid_only(tmp_path: Path):
    seg1 = segment([(1, VID, b"a"), (2, OTHER, b"x"), (3, VID, b"c")])
    seg2 = segment([(1, VID, b"b")], first_lsn=3)
    assert sd.vid_bodies(lane(tmp_path / "l", seg1, seg2)) == {1: b"b", 3: b"c"}


def test_torn_tail_is_ignored(tmp_path: Path):
    seg = segment([(1, VID, b"a"), (2, VID, b"bb")])
    assert sd.vid_bodies(lane(tmp_path / "l", seg[:-1])) == {1: b"a"}


def test_bad_segment_header_raises(tmp_path: Path):
    with pytest.raises(ValueError, match="bad segment header"):
        sd.vid_bodies(lane(tmp_path / "l", b"junk" * 20))


def test_main_reports_and_exits_zero_when_equal(tmp_path: Path, capsys):
    orig = lane(tmp_path / "o", segment([(1, VID, b"a"), (2, VID, b"b")]))
    healed = lane(tmp_path / "h", segment([(1, VID, b"a"), (3, VID, b"c")]))
    assert sd.main([str(orig), str(healed)]) == 0
    assert capsys.readouterr().out == "equal=1 differ=0 orig_only=1 healed_only=1\n"


def test_main_exits_one_on_differing_body(tmp_path: Path, capsys):
    orig = lane(tmp_path / "o", segment([(1, VID, b"a")]))
    healed = lane(tmp_path / "h", segment([(1, VID, b"z")]))
    assert sd.main([str(orig), str(healed)]) == 1
    assert capsys.readouterr().out == "equal=0 differ=1 orig_only=0 healed_only=0\n"
