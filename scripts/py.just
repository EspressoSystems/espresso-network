# Scripts with uv inline dependencies, type-checked in their own environment.
uv_scripts := "bench-block-build"
scripts := "nextest-ci compile-metrics release test_compile_metrics.py test_nextest_ci.py test_release.py network-bench/bench network-bench/test_bench.py"

fmt *args:
    ruff format {{scripts}} {{uv_scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{uv_scripts}} {{args}}

check *args:
    ty check {{scripts}} {{args}}
    for s in {{uv_scripts}}; do uv sync --script "$s" && ty check --python "$(uv python find --script "$s")" "$s" {{args}}; done

test *args:
    python3 -m unittest discover -p "test_*.py" {{args}}
    python3 -m unittest discover -s network-bench -p "test_*.py" {{args}}
