.PHONY: build release graphs test

build:
	@cargo build

release:
	@cargo build --release

perf.csv: release
	@cargo run --release --bin perf

graphs: perf.csv
	@python3 graphs.py

test: release
	@cargo run --release --bin test