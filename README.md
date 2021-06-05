# WARNING: This repo now lives here: https://gitlab.com/soenkehahn/touchscreen-instrument

# touchscreen-instrument

This is a musical instrument running on linux using an external monitor with
touchscreen support. It currently works with the `gechic 1503i` monitor.

Here's a video of a performance using the instrument by @caroline-lin, @dan-f
and @soenkehahn: https://youtu.be/9unGIcbHJ0A

## development

You'll need [rustup](https://rustup.rs/) and a few dependencies as listed
[here](https://github.com/soenkehahn/touchscreen-instrument/blob/master/ansible/tasks.yaml#L3).

Run the test-suite with:

`cargo test`

Run the debug version:

`cargo run`

There is a command line interface, see:

`cargo run -- --help`

If you don't have a touchscreen attached to your computer, you can still run
just the ui of the program with:

`cargo run -- --dev-mode`

Build the release version:

`cargo build --release`

Install and run the release version:

`cargo install --force && touchscreen-instrument`

## raspberry pi

There's an ansible script that sets up the `touchscreen-instrument` on a
raspberry pi. It assumes that raspbian is installed and that the device can be
accessed with ssh through `pi@raspberrypi.local`. Run the deployment with:

`make deploy`

This'll take some time on the first run. It compiles the program on the
raspberry.
