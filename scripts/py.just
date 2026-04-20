scripts := "nextest-history nextest-summary"

fmt *args:
    ruff format {{scripts}} {{args}}

lint *args:
    ruff check {{scripts}} {{args}}

check *args:
    ty check {{scripts}} {{args}}
