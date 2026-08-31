scripts := "nextest-ci compile-metrics test_compile_metrics.py llvm-lines test_llvm_lines.py"

fmt *args:
    ruff format {{scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{args}}

check *args:
    ty check {{scripts}} {{args}}

test *args:
    python3 -m unittest discover -p "test_*.py" {{args}}
