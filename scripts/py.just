scripts := "nextest-history nextest-summary _nextest_parse.py"

fmt *args:
    ruff format {{scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{args}}

check *args:
    ty check {{scripts}} {{args}}
