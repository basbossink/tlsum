alias b := build
alias br := build-release
alias t := test
alias w := watch
alias tc := test-coverage

build:
	cargo build

build-release: 
	RUSTFLAGS="-C target-cpu=native" cargo build --release

crit:
	RUSTFLAGS="-C target-cpu=native" CARGO_MANIFEST_DIR="{{justfile_directory()}}" cargo criterion

flame:
	cargo flamegraph

test: 
	cargo nextest run -j 1

watch:
	watchexec -c -e rs -- cargo nextest run -j 1

clippy:
	cargo clippy --all -- -W clippy::all -W clippy::pedantic -W clippy::restriction -W clippy::nursery -D warnings

test-coverage:
    cargo tarpaulin --skip-clean
