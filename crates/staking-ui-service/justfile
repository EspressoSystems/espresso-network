default:
    just --list

doc *args:
    cargo doc --no-deps --document-private-items {{args}}

fmt:
    cargo fmt --all

fix *args:
    cargo clippy --fix {{args}}

lint *args:
    cargo clippy {{args}} -- -D warnings

build profile="dev" features="":
    cargo build --profile {{profile}} {{features}}

test *args:
    cargo nextest run --locked --workspace --verbose {{args}}
