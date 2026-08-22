default: check

run:
    cargo run

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo test --all-targets --all-features

check: fmt-check lint test
