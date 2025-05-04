.PHONY: build perf release graphs test

build:
	@cargo build

release:
	@cargo build --release

perf: release
	@cargo run --release --bin perf

graphs: perf
	@python3 graphs.py

test: release
	@cargo run --release --bin test