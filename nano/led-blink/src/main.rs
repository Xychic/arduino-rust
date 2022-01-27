#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);

    

    ufmt::uwriteln!(&mut serial, "Hello from Arduino!").void_unwrap();

    pins.d9.into_output();
    
    peripherals.TC1.tccr1a.write(|w| w.wgm1().bits(0b01).com1a().match_clear());
    peripherals.TC1.tccr1b.write(|w| w.wgm1().bits(0b01).cs1().prescale_64());

    let mut duty = 0;
    let step = 10;
    let steps = 255 / step;

    loop {
        for _ in 0..steps {
            ufmt::uwriteln!(&mut serial, "Duty: {}", duty).void_unwrap();
            duty += step;
            peripherals.TC1.ocr1a.write(|w| unsafe { w.bits(duty) });
            arduino_hal::delay_ms(20);
        }
        for _ in 0..steps {
            ufmt::uwriteln!(&mut serial, "Duty: {}", duty).void_unwrap();
            duty -= step;
            peripherals.TC1.ocr1a.write(|w| unsafe { w.bits(duty) });
            arduino_hal::delay_ms(20);
        }
    }
}
