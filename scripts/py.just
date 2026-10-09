# Scripts with uv inline dependencies, type-checked in their own environment.
uv_scripts := "bench-block-build"
scripts := "nextest-ci compile-metrics release test_compile_metrics.py test_nextest_ci.py test_release.py network-bench/bench network-bench/netbench.py network-bench/nullserver.py network-bench/tests/test_nullserver.py network-bench/tests/test_bench.py network-bench/tests/test_netbench.py network-bench/tests/fakes.py network-bench/tests/conftest.py network-bench/aws-bench network-bench/tests/test_aws_plan.py network-bench/tests/test_aws_run.py network-bench/tests/test_aws_chaos.py network-bench/tests/test_aws_chaos_report.py network-bench/tests/test_chaos.py network-bench/chaos.py network-bench/tests/test_aws_collect.py network-bench/tests/test_aws_status.py network-bench/tests/test_aws_fleet_up.py network-bench/tests/test_aws_rds.py network-bench/tests/test_aws_publish.py network-bench/tests/test_aws_latency.py network-bench/latency.py network-bench/tests/test_latency.py network-bench/leadertrace.py network-bench/trace-plots network-bench/tests/test_leadertrace.py network-bench/tests/test_trace_plots.py network-bench/throughput-plot network-bench/tests/test_throughput_plot.py network-bench/mainnet-locations network-bench/tests/test_mainnet_locations.py"
pytest := "uv run --python 3.14 --with pytest --with pytest-xdist --with pytest-timeout --with matplotlib pytest"

fmt *args:
    ruff format {{scripts}} {{uv_scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{uv_scripts}} {{args}}

check *args:
    ty check --extra-search-path network-bench --extra-search-path network-bench/tests --extra-search-path "$(uv run -q --with pytest --with matplotlib python -c 'import pathlib, pytest; print(pathlib.Path(pytest.__file__).parents[1])')" {{scripts}} {{args}}
    for s in {{uv_scripts}}; do uv sync --script "$s" && ty check --python "$(uv python find --script "$s")" "$s" {{args}}; done

test *args:
    {{pytest}} {{args}}

# Needs tofu on PATH and internet for `tofu init`, and the repo `.env`.
test-slow *args:
    SLOW_TESTS=1 {{pytest}} -m slow {{args}}
