#![no_std]
#![no_main]

use arduino_hal::hal::port::{PD6, PD3, PB1, PD5, PB3, PB2};
use arduino_hal::pac::{TC1, TC0, TC2};
use arduino_hal::port::mode::Output;
use arduino_hal::port::Pin;
use panic_halt as _;
use arduino_hal::{prelude::*, Peripherals};

struct PWMController<T, U> {
    timer: T,
    duty: U,
}

impl PWMController<TC0, u8> {

}

impl PWMController<TC1, u16> {
    fn new() -> PWMController<TC1, u16> {
        let timer = arduino_hal::Peripherals::take().unwrap().TC1;
        timer.tccr1a.write(|w| w.wgm1().bits(0b01).com1a().match_clear());
        timer.tccr1b.write(|w| w.wgm1().bits(0b01).cs1().prescale_64());
        PWMController {timer, duty:0}
    }

    fn inc_duty(&mut self, amt: u16) {
        self.duty += amt;
    }

    fn dec_duty(&mut self, amt: u16) {
        self.duty -= amt;
    }
    
    fn set_duty(&mut self, amt: u16) {
        self.duty = amt;
    }

    fn update(&self) {
        self.timer.ocr1a.write(|w| unsafe { w.bits(self.duty) });
    }
    
}

impl PWMController<TC2, u8> {
    
}

trait PWM<T, U> {
    fn get_controller(self) -> PWMController<T, U>;
}

impl<T> PWM<TC1, u16> for Pin<T, PB1> {
    fn get_controller(self) -> PWMController<TC1, u16> {
        PWMController::new()
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 9600);

    ufmt::uwriteln!(&mut serial, "Hello from Arduino!").void_unwrap();

    // let mut led = pins.d6.into_output();
    let led = pins.d9.into_output();
    
    let tc1 = dp.TC1;
    tc1.tccr1a.write(|w| w.wgm1().bits(0b01).com1a().match_clear());
    tc1.tccr1b.write(|w| w.wgm1().bits(0b01).cs1().prescale_64());

    let mut duty = 0;
    let step = 10;
    let steps = 255 / step;

    loop {
        for _ in 0..steps {
            ufmt::uwriteln!(&mut serial, "Duty: {}", duty).void_unwrap();
            duty += step;
            tc1.ocr1a.write(|w| unsafe { w.bits(duty as u16) });
            arduino_hal::delay_ms(20);
        }
        for _ in 0..steps {
            ufmt::uwriteln!(&mut serial, "Duty: {}", duty).void_unwrap();
            duty -= step;
            tc1.ocr1a.write(|w| unsafe { w.bits(duty as u16) });
            arduino_hal::delay_ms(20);
        }
        


        // blink_n(&mut led, 2, 100);
        // arduino_hal::delay_ms(1000);
    }
}
