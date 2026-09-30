scripts := "nextest-ci compile-metrics release test_compile_metrics.py test_nextest_ci.py test_release.py network-bench/bench network-bench/netbench.py network-bench/test_bench.py network-bench/test_netbench.py network-bench/fakes.py network-bench/test_fakes.py network-bench/aws-bench network-bench/test_aws_bench.py network-bench/test_aws_fleet.py timed_tests.py test_timed_tests.py"

fmt *args:
    ruff format {{scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{args}}

check *args:
    ty check --extra-search-path network-bench {{scripts}} {{args}}

# Arguments: -k SUBSTR (repeatable), -s, -p, --slowest, --max-test-s; not unittest's -v or test ids.
# The per-test limit is 5 s: the `LoadTest` staircase cases need STEP_S = 8 virtual seconds at scale 20.
test *args:
    python3 timed_tests.py --max-test-s 5 {{args}}
    python3 timed_tests.py -s network-bench --max-test-s 5 {{args}}

# Per-test durations, no limit.
test-timing *args:
    python3 timed_tests.py --slowest 40 {{args}}
    python3 timed_tests.py -s network-bench --slowest 40 {{args}}

# Needs tofu on PATH and internet for `tofu init`, and the repo `.env`.
test-slow *args:
    SLOW_TESTS=1 python3 timed_tests.py -s network-bench -k Tofu -k real_env_file {{args}}
