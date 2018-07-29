#!/bin/bash

# from https://wiki.linuxaudio.org/wiki/raspberrypi
export DBUS_SESSION_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket
jackd -dalsa &

RUST_APP=`which rust-device-reading`

xinit $RUST_APP --layout Parallelograms &

sleep 3 

# connect the clients 
SYSTEM_LEFT="system:playback_1"
SYSTEM_RIGHT="system:playback_2"

RUST_LEFT="rust-device-reading:left-output"
RUST_RIGHT="rust-device-reading:right-output"

jack_connect $SYSTEM_LEFT $RUST_LEFT
jack_connect $SYSTEM_RIGHT $RUST_RIGHT

# todo: cleanup



