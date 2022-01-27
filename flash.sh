#! /usr/bin/zsh

set -e

cargo build --bin $1
avrdude -p m328p -c arduino -P /dev/ttyUSB0 -v -U flash:w:./target/avr-atmega328p/debug/$1.elf:e