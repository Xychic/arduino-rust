#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::{
    delay_ms,
    hal::port::Dynamic,
    port::{
        mode::{Input, PullUp},
        Pin,
    },
    prelude::*,
};
use core::cell;
use panic_halt as _;

const PRESCALER: u32 = 1024;
const TIMER_COUNTS: u32 = 125;

const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16000;

static MILLIS_COUNTER: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

fn millis_init(tc0: arduino_hal::pac::TC0) {
    // Configure the timer for the above interval (in CTC mode)
    // and enable its interrupt.
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a.write(|w| unsafe { w.bits(TIMER_COUNTS as u8) });
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });
}

#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

fn millis() -> u32 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}
struct Button<'a> {
    pin: &'a Pin<Input<PullUp>, Dynamic>,
    state: bool,
    last_down: u32,
    last_up: u32,
    bump_duration: u32,
}

impl Button<'_> {
    fn new(pin: &Pin<Input<PullUp>, Dynamic>) -> Button {
        Button {
            pin,
            state: false,
            last_down: 0,
            last_up: 0,
            bump_duration: u32::MAX,
        }
    }

    fn update(&mut self) {
        if self.pin.is_low() {
            if !self.state {
                self.state = true;
                self.last_down = millis();
            }
        } else if self.state {
            self.state = false;
            self.last_up = millis();

            self.bump_duration = self.last_up - self.last_down;
            if self.last_up < self.last_down {
                self.bump_duration += u32::MAX;
            }
        }
    }

    fn is_pressed(&mut self) -> bool {
        self.state
    }

    fn is_held(&self, duration: u32) -> bool {
        if self.state {
            let current = millis();
            let mut diff = current - self.last_down;
            if current < self.last_down {
                diff += u32::MAX;
            }
            return diff > duration;
        }
        false
    }

    fn was_bumped(&mut self, duration: u32) -> bool {
        let bumped = self.bump_duration < duration;
        self.bump_duration = u32::MAX;
        bumped
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);
    let mut yellow_led_pin = pins.d9.into_output();
    let mut red_led_pin = pins.d10.into_output();
    let mut green_led_pin = pins.d11.into_output();
    let button_pin = pins.d8.into_pull_up_input().downgrade();
    let mut button = Button::new(&button_pin);

    // For millis to work
    millis_init(peripherals.TC0);
    unsafe { avr_device::interrupt::enable() };

    ufmt::uwriteln!(&mut serial, "Finished Setup!").void_unwrap();

    loop {
        button.update();

        // led_pin.toggle();
        // arduino_hal::delay_ms(1000);
        // ufmt::uwriteln!(&mut serial, "Button pressed: {}", button_pin.is_low()).void_unwrap();
        ufmt::uwriteln!(&mut serial, "Time: {}", millis()).void_unwrap();
        ufmt::uwriteln!(
            &mut serial,
            "Status: {}, Last Down: {}, Last Up: {}",
            button.state,
            button.last_down,
            button.last_up
        )
        .void_unwrap();

        if button.is_pressed() {
            yellow_led_pin.set_high();
        } else {
            yellow_led_pin.set_low();
        }

        if button.is_held(1000) {
            green_led_pin.set_high();
        } else {
            green_led_pin.set_low();
        }

        if button.was_bumped(1000) {
            red_led_pin.set_high();
        } else {
            red_led_pin.set_low();
        }

        delay_ms(50);
    }
}
