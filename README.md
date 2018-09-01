# touchscreen-instrument

This is a musical instrument running on linux using an external monitor with
touchscreen support. It currently works with the `gechic 1503i` monitor.

## development

You'll need [rustup](https://rustup.rs/).

Run the test-suite with:

`cargo test`

Run the debug version:

`cargo run`

Build the release version:

`cargo build --release`

Install and run the release version:

`cargo install --force && touchscreen-instrument`



# running on raspberry pi

## setup

- install rustup
- apt-get install jackd2 libjack-jackd2-dev libsdl2-dev libsdl2-gfx-dev xinit
- cargo install --force
- follow this guide: https://wiki.linuxaudio.org/wiki/raspberrypi
- disable_overscan=1 in /boot/config.txt
- set `allowed_users=anybody` in /etc/X11/Xwrapper.config

## starting the instrument

`make debug`
