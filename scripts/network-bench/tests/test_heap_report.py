import os
from pathlib import Path

import pytest
from fakes import load_script

report = load_script("heap-report")

# One r-x LOAD whose vaddr is 0x10000 above its file offset, like our PIE node binaries.
READELF = """\
Program Headers:
  Type           Offset   VirtAddr           PhysAddr           FileSize MemSize  Flags Align
  LOAD           0x000000 0x0000000000000000 0x0000000000000000 0x001000 0x001000 R   0x1000
  LOAD           0x002000 0x0000000000012000 0x0000000000012000 0x001000 0x001000 R E 0x1000
  GNU_STACK      0x000000 0x0000000000000000 0x0000000000000000 0x000000 0x000000 RW  0x10
"""
NM = """\
0000000000012000 T _rjem_malloc
0000000000012100 t node::spawn_lane::h0123456789abcdef
0000000000012200 T node::other
0000000000012300 d some_data
"""
# Runtime base 0x7000: the r-x segment maps file offset 0x2000 at 0x9000.
DUMP = """\
heap_v2/524288
  t*: 3: 300 [0: 0]
@ 0x9150 0x9250
  t*: 2: 200 [0: 0]
  t0: 2: 200 [0: 0]
@ 0x9150
  t*: 1: 100 [0: 0]
@ 0x9150
  t*: 0: 0 [0: 0]

MAPPED_LIBRARIES:
7000-8000 r--p 00000000 00:30 1                     /usr/bin/espresso-node-sqlite
9000-a000 r-xp 00002000 00:30 1                     /usr/bin/espresso-node-sqlite
b000-c000 r--p 00000000 00:00 0
"""


def touch(path: Path, mtime: int) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.touch()
    os.utime(path, (mtime, mtime))
    return path


def symbolizer():
    mappings = report.parse_mappings(DUMP)
    return report.make_symbolizer(
        mappings, report.parse_segments(READELF), report.parse_symbols(NM)
    )


def test_the_newest_dump_wins_by_mtime(tmp_path: Path) -> None:
    old = touch(tmp_path / "heap.2.9.i9.heap", 100)
    new = touch(tmp_path / "heap.1.0.f.heap", 200)
    assert report.newest_dump([old, new]) == new


def test_segments_carry_the_vaddr_offset_delta() -> None:
    assert report.parse_segments(READELF)[1] == report.Segment(0x2000, 0x3000, 0x10000)


def test_symbols_keep_code_only() -> None:
    addrs, names = report.parse_symbols(NM)
    assert addrs == [0x12000, 0x12100, 0x12200]
    assert names[1] == "node::spawn_lane::h0123456789abcdef"


def test_mapped_binary_is_the_node_binary() -> None:
    assert report.mapped_binary(report.parse_mappings(DUMP)) == "espresso-node-sqlite"


def test_mapped_binary_requires_exactly_one() -> None:
    with pytest.raises(SystemExit):
        report.mapped_binary([])


def test_stacks_skip_empty_and_use_the_total_line() -> None:
    assert report.parse_stacks(DUMP) == [
        report.Stack([0x9150, 0x9250], 200),
        report.Stack([0x9150], 100),
    ]


def test_symbolize_goes_through_file_offset_and_vaddr() -> None:
    sym = symbolizer()
    # 0x9150 -> offset 0x2150 -> vaddr 0x12150, inside spawn_lane (return address - 1).
    assert sym(0x9150) == "node::spawn_lane"
    assert sym(0x9250) == "node::other"
    assert sym(0x7010) is None
    assert sym(0xB100) is None


def test_aggregate_attributes_flat_to_first_non_allocator_frame() -> None:
    names = {
        1: "_rjem_malloc",
        2: "alloc::raw_vec::finish_grow",
        3: "app::caller",
        4: "app::main",
    }
    result = report.aggregate(
        [report.Stack([1, 2, 3, 4], 200), report.Stack([4], 100)], names.get
    )
    assert result.total == 300
    assert dict(result.flat) == {"app::caller": 200, "app::main": 100}
    assert result.cumulative["app::main"] == 300
    assert result.cumulative["_rjem_malloc"] == 200


def test_render_lists_total_and_both_tables() -> None:
    result = report.Report(2 * 2**20, {"a": 2 * 2**20}, {"a": 2 * 2**20, "b": 2**20})
    lines = report.render(result, top=1).splitlines()
    assert lines[0] == "total in use (sampled estimate): 2 MiB"
    assert "     2.0 MiB 100.0%  a" in lines
    assert not any(line.endswith("  b") for line in lines)


@pytest.mark.parametrize(
    "frame",
    [
        "<alloc::raw_vec::RawVecInner>::finish_grow",
        "do_rallocx",
        "<alloc::raw_vec::RawVec<u8>>::grow_one",
    ],
)
def test_vec_growth_frames_are_allocator_frames(frame: str) -> None:
    assert report.ALLOCATOR_FRAME.match(frame)
