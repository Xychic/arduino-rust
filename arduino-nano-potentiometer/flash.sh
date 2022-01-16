#! /usr/bin/zsh

set -e

cargo fmt
cargo build
avrdude -p m328p -c arduino -P /dev/ttyUSB0 -v -U flash:w:$(find ./target/avr-atmega328p/debug/*.elf):e