#!/usr/bin/env bash

set -o errexit

# from https://wiki.linuxaudio.org/wiki/raspberrypi
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket

xinit $PWD/target/release/rust-device-reading \
    --layout Parallelograms \
    --volume 5
