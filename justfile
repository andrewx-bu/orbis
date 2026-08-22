default: check

run:
    cargo run --locked

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --locked --all-targets --all-features -- -D warnings

test:
    cargo test --locked --all-targets --all-features

check: fmt-check lint test
