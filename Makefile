# ARM Cortex-M0 Superscalar Processor Simulator
# Requires: Rust toolchain (cargo + rustc), install from https://rustup.rs

.PHONY: all build run clean experiment

all: build

build:
	cargo build --release

run: build
	cargo run --release -- $(ARGS)

experiment: build
	cargo run --release -- --experiment

clean:
	cargo clean
