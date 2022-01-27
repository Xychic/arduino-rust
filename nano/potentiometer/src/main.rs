#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);
    let mut adc = arduino_hal::Adc::new(peripherals.ADC, Default::default());

    let analog_pin = pins.a0.into_analog_input(&mut adc);
    let _led_pin = pins.d9.into_output();

    ufmt::uwriteln!(&mut serial, "Hello from Arduino!").void_unwrap();

    peripherals
        .TC1
        .tccr1a
        .write(|w| w.wgm1().bits(0b01).com1a().match_clear());
    peripherals
        .TC1
        .tccr1b
        .write(|w| w.wgm1().bits(0b01).cs1().prescale_256());

    loop {
        let pot_val = adc.read_blocking(&analog_pin);
        let pwm_val = pot_val / 4;

        ufmt::uwriteln!(&mut serial, "Pot: {}", pot_val).void_unwrap();
        ufmt::uwriteln!(&mut serial, "PWM Val: {}", pwm_val).void_unwrap();

        peripherals.TC1.ocr1a.write(|w| unsafe { w.bits(pwm_val) });
        arduino_hal::delay_ms(100);
    }
}
