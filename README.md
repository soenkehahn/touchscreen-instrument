# setup on raspberry

- apt-get install jackd2 libjack-jackd2-dev libsdl2-dev libsdl2-gfx-dev xinit
- cargo install --force
- follow this guide: https://wiki.linuxaudio.org/wiki/raspberrypi
- disable_overscan=1 in /boot/config.txt
- set `allowed_users=anybody` in /etc/X11/Xwrapper.config
