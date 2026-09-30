scripts := "nextest-ci compile-metrics release test_compile_metrics.py test_nextest_ci.py test_release.py network-bench/bench network-bench/netbench.py network-bench/test_bench.py network-bench/test_netbench.py network-bench/aws-bench network-bench/test_aws_bench.py network-bench/test_aws_fleet.py"

fmt *args:
    ruff format {{scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{args}}

check *args:
    ty check --extra-search-path network-bench {{scripts}} {{args}}

test *args:
    python3 -m unittest discover -p "test_*.py" {{args}}
    python3 -m unittest discover -s network-bench -p "test_*.py" {{args}}
