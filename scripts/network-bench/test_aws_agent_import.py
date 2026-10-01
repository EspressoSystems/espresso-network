import shutil
from pathlib import Path

from fakes import SCRIPT, load_script


def test_agent_host_imports_without_the_repo_env(tmp_path: Path):
    """Hosts get `aws-bench` and `netbench.py` alone in /opt/bench, where no `.env` exists."""
    bench = tmp_path / "opt" / "bench"
    bench.mkdir(parents=True)
    shutil.copy(SCRIPT, bench / "aws-bench")
    shutil.copy(SCRIPT.with_name("netbench.py"), bench / "netbench.py")

    module = load_script("aws-bench", bench)

    args = module.parse_args(["agent-drive", "a.json", "out"])
    assert args.command == "agent-drive"
