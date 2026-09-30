scripts := "nextest-ci compile-metrics release test_compile_metrics.py test_nextest_ci.py test_release.py network-bench/bench network-bench/netbench.py network-bench/test_bench.py network-bench/test_netbench.py network-bench/fakes.py network-bench/conftest.py network-bench/aws-bench network-bench/test_aws_plan.py network-bench/test_aws_run.py network-bench/test_aws_collect.py network-bench/test_aws_status.py network-bench/test_aws_fleet_up.py network-bench/test_aws_rds.py"
pytest := "uv run --with pytest --with pytest-xdist --with pytest-timeout pytest"

fmt *args:
    ruff format {{scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{args}}

check *args:
    ty check --extra-search-path network-bench --extra-search-path "$(uv run -q --with pytest python -c 'import pathlib, pytest; print(pathlib.Path(pytest.__file__).parents[1])')" {{scripts}} {{args}}

test *args:
    {{pytest}} {{args}}

# Needs tofu on PATH and internet for `tofu init`, and the repo `.env`.
test-slow *args:
    SLOW_TESTS=1 {{pytest}} -m slow {{args}}
