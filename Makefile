debug: build debug_run

debug_run:
	./raspberry_debug.sh

build:
	cargo build --release
