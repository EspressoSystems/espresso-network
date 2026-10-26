import subprocess
from pathlib import Path

import pytest

SCRIPT = Path(__file__).parent / "check-doc-patch"


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-c", "user.name=t", "-c", "user.email=t@t", *args],
        cwd=repo,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    git(tmp_path, "init", "-q")
    (tmp_path / "doc").mkdir()
    (tmp_path / "doc/a.md").write_text("a\n")
    (tmp_path / "AGENTS.md").write_text("b\n")
    (tmp_path / "other.txt").write_text("o\n")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-qm", "init")
    return tmp_path


def check(repo: Path) -> subprocess.CompletedProcess:
    git(repo, "add", "-A")
    patch = repo.parent / f"{repo.name}.patch"
    patch.write_text(git(repo, "diff", "--cached", "--binary"))
    return subprocess.run(
        [str(SCRIPT), str(patch)], capture_output=True, text=True, cwd=repo, check=False
    )


def test_modify_only_accepted(repo: Path):
    (repo / "doc/a.md").write_text("changed\n")
    (repo / "AGENTS.md").write_text("changed\n")
    assert check(repo).returncode == 0


def test_create_accepted(repo: Path):
    (repo / "doc/new.md").write_text("n\n")
    assert check(repo).returncode == 0


def test_delete_accepted(repo: Path):
    (repo / "doc/a.md").unlink()
    assert check(repo).returncode == 0


def test_empty_rejected(repo: Path):
    assert check(repo).returncode == 1


def test_rename_rejected(repo: Path):
    (repo / "doc/a.md").rename(repo / "doc/b.md")
    assert check(repo).returncode == 1


def test_symlink_rejected(repo: Path):
    (repo / "doc/link").symlink_to("/etc/passwd")
    assert check(repo).returncode == 1


def test_executable_create_rejected(repo: Path):
    (repo / "doc/run.sh").write_text("#!/bin/sh\n")
    (repo / "doc/run.sh").chmod(0o755)
    assert check(repo).returncode == 1


def test_mode_change_rejected(repo: Path):
    (repo / "doc/a.md").chmod(0o755)
    assert check(repo).returncode == 1


def test_binary_rejected(repo: Path):
    (repo / "doc/blob.bin").write_bytes(bytes(range(256)))
    assert check(repo).returncode == 1


def test_path_outside_rejected(repo: Path):
    (repo / "other.txt").write_text("changed\n")
    r = check(repo)
    assert r.returncode == 1
    assert "other.txt" in r.stderr
