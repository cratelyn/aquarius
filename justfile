# ======================== development commands ============================= #

# choose a command if given no other arguments.
default:
    just --choose

# === run commands === #

run-demo:
    ./run.sh

run-test-server:
    cargo run --package aquarius-test-server

run-load-test:
    cargo run -- --trace --total 512 --rate 128 localhost:8080

# === common commands === #

build:
    cargo build --all-features --all-targets

check:
    cargo check --all-features --all-targets

check-short:
    cargo check --all-features --all-targets --message-format=short

doc:
    cargo doc

doc-open:
    cargo doc --open

doc-test:
    cargo test --doc

lint:
    cargo clippy --all-targets

test:
    cargo nextest run --all-features --all-targets

test-all: test doc-test

# === ci: build, document, test, and lint

ci: build doc test-all lint

# === watch command output === #

watch-check:
    cargo watch --clear --why --shell 'just check'

watch-check-short:
    cargo watch --clear --why --shell 'just check-short'

watch-test:
    cargo watch --clear --why --shell 'just test'

watch-ci:
    cargo watch --clear --why --shell 'just ci'

watch-demo:
    cargo watch --clear --why --shell 'just run-demo'
