import os
from pathlib import Path

from fakes import load_script

report = load_script("heap-report")


def touch(path: Path, mtime: int) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.touch()
    os.utime(path, (mtime, mtime))
    return path


def test_the_newest_dump_wins_by_mtime(tmp_path: Path) -> None:
    old = touch(tmp_path / "heap.2.9.i9.heap", 100)
    new = touch(tmp_path / "heap.1.0.f.heap", 200)
    assert report.newest_dump([old, new]) == new


def test_node0_uses_the_sqlite_binary_when_collected(tmp_path: Path) -> None:
    assert report.binary_for(tmp_path, "node0") == tmp_path / "espresso-node.bin"
    touch(tmp_path / "espresso-node-sqlite.bin", 1)
    assert report.binary_for(tmp_path, "node0") == tmp_path / "espresso-node-sqlite.bin"
    assert report.binary_for(tmp_path, "node1") == tmp_path / "espresso-node.bin"


def test_jeprof_argv(tmp_path: Path) -> None:
    assert report.jeprof_argv(Path("b"), Path("d.heap"), "svg") == [
        "jeprof",
        "--svg",
        "--show_bytes",
        "b",
        "d.heap",
    ]
