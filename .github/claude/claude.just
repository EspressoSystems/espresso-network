# claude-gh uv project; just runs module recipes in this directory.

[script("../../scripts/with-tools", "python", "uv")]
fmt *args:
    uv run --locked ruff format . {{args}}

[script("../../scripts/with-tools", "python", "uv")]
lint *args:
    uv run --locked ruff check . {{args}}

[script("../../scripts/with-tools", "python", "uv")]
check *args:
    uv run --locked ty check . {{args}}

[script("../../scripts/with-tools", "python", "uv")]
test *args:
    uv run --locked pytest {{args}}

# Replay the review on cases in eval/cases.toml; see README.md.
[script("../../scripts/with-tools", "python", "uv")]
eval *args:
    cd ../.. && PYTHONPATH=.github/claude uv run --locked --project .github/claude python .github/claude/eval/eval.py {{args}}
