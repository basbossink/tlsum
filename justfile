alias b := build
alias br := build-release
alias t := test
alias w := watch

build:
	cargo build

build-release: 
	RUSTFLAGS="-C target-cpu=native" cargo build --release

crit:
	RUSTFLAGS="-C target-cpu=native" CARGO_MANIFEST_DIR="{{justfile_directory()}}" cargo build --release

flame:
	cargo flamegraph

test: 
	cargo test

watch:
	watchexec -c -e rs cargo test

clippy:
	cargo clippy --all -- -W clippy::all -W clippy::pedantic -W clippy::restriction -W clippy::nursery -D warnings
