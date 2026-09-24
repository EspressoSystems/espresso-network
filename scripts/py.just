scripts := "nextest-ci compile-metrics release test_compile_metrics.py test_nextest_ci.py test_release.py"

fmt *args:
    ruff format {{scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{args}}

check *args:
    ty check {{scripts}} {{args}}

test *args:
    python3 -m unittest discover -p "test_*.py" {{args}}
