debug: build debug_run

debug_run:
	./raspberry_debug.sh

debug_stop:
	pkill xinit

build:
	cargo build --release
