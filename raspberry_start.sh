#!/usr/bin/env bash

set -o errexit

# from https://wiki.linuxaudio.org/wiki/raspberrypi
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket
jackd -dalsa &

RUST_APP=$(which rust-device-reading)
xinit $RUST_APP --layout Parallelograms &
sleep 3

# connect the clients
jack_connect "system:playback_1" "rust-device-reading:left-output"
jack_connect "system:playback_2" "rust-device-reading:right-output"
