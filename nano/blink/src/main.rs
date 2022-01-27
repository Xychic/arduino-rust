#![no_std]
#![no_main]

use arduino_hal::hal::port::PB5;
use arduino_hal::port::mode::Output;
use arduino_hal::port::Pin;
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut led = pins.d13.into_output();

    let mut blinks = 1;
    let limit = 3;

    loop {
        blink_n(&mut led, blinks, 100);
        arduino_hal::delay_ms(1000);
        blinks += 1;
        if blinks > limit {
            blinks -= limit;
        }
    }
}

fn blink_n(led: &mut Pin<Output, PB5>, times: i32, delay: u16) {
    for _ in 0..times * 2 { // *2 for toggle on and off
        led.toggle();
        arduino_hal::delay_ms(delay);
    }
}
