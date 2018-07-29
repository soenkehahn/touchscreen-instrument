#!/usr/bin/bash

# from https://wiki.linuxaudio.org/wiki/raspberrypi
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket
jackd -dalsa &


rust-device-reading --layout Parallelograms &

sleep 1

# connect the clients 
SYSTEM_LEFT = "system:playback_1"
SYSTEM_RIGHT = "system:playback_2"

RUST_LEFT = "rust-device-reading:left-output"
RUST_RIGHT = "rust-device-reading:right-output"

jack_connect $RUST_LEFT $SYSTEM_LEFT
jack_connect $RUST_RIGHT $SYSTEM_RIGHT

# todo: cleanup



