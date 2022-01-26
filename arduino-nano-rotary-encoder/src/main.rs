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
use avr_device::interrupt::Mutex;
use panic_halt as _;

use core::{
    cell::Cell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

#[allow(dead_code)]
enum PWMAccuracy {
    LOW,
    MEDIUM,
    HIGH,
}

impl PWMAccuracy {
    fn val(&self) -> u16 {
        match self {
            PWMAccuracy::LOW => 255,
            PWMAccuracy::MEDIUM => 511,
            PWMAccuracy::HIGH => 1023,
        }
    }
}

static ROTARY_CHANGE: AtomicBool = AtomicBool::new(false);
static TEMP: Mutex<Cell<u16>> = Mutex::new(Cell::new(0));
static BRIGHTNESS: Mutex<Cell<u16>> = Mutex::new(Cell::new(1));
static ROTARY_PINS: Mutex<Cell<MaybeUninit<[Pin<Input<PullUp>, Dynamic>; 3]>>> =
    Mutex::new(Cell::new(MaybeUninit::uninit()));
static MILLIS_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

const PWM_ACCURACY: PWMAccuracy = PWMAccuracy::HIGH;
const TEMP_STEP: u16 = 25;
const BRIGHTNESS_STEP: u16 = 25;

const PRESCALER: u32 = 1024;
const TIMER_COUNTS: u32 = 125;
const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16000;

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);

    let rotary_pins = [
        pins.d8.into_pull_up_input().downgrade(),
        pins.d2.into_pull_up_input().downgrade(),
        pins.d7.into_pull_up_input().downgrade(),
    ];

    avr_device::interrupt::free(|cs| {
        ROTARY_PINS.borrow(cs).set(MaybeUninit::new(rotary_pins));
    });

    let _red_led_pin = pins.d9.into_output();
    let _green_led_pin = pins.d10.into_output();

    let timer1 = peripherals.TC1;
    timer1.tccr1a.write(|w| {
        w.wgm1()
            .bits(match PWM_ACCURACY {
                PWMAccuracy::LOW => 0b01,
                PWMAccuracy::MEDIUM => 0b10,
                PWMAccuracy::HIGH => 0b11,
            })
            .com1a()
            .match_clear()
            .com1b()
            .match_clear()
    });
    timer1.tccr1b.write(|w| match PWM_ACCURACY {
        PWMAccuracy::LOW => w.wgm1().bits(0b00).cs1().prescale_256(),
        PWMAccuracy::MEDIUM => w.wgm1().bits(0b00).cs1().prescale_256(),
        PWMAccuracy::HIGH => w.wgm1().bits(0b00).cs1().prescale_64(),
    });

    peripherals.EXINT.pcicr.write(|w| unsafe { w.bits(0b100) });
    peripherals.EXINT.pcmsk2.write(|w| unsafe { w.bits(0b100) });

    millis_init(peripherals.TC0);

    let mut prev_button_state = false;
    let mut powered = false;
    let mut last_up = 0;
    let mut last_down = 0;

    unsafe {
        avr_device::interrupt::enable();
    }

    ufmt::uwriteln!(&mut serial, "Finished Setup!").void_unwrap();
    loop {
        // if changed(&TMR_OVERFLOW) {
        //     ufmt::uwriteln!(&mut serial, "Timer!").void_unwrap();
        // }
        if changed(&ROTARY_CHANGE) {
            let temp = get_from_mutex(&TEMP);
            let brightness = get_from_mutex(&BRIGHTNESS);
            let red =
                PWM_ACCURACY.val().min(temp) as u32 * brightness as u32 / PWM_ACCURACY.val() as u32;
            let green = PWM_ACCURACY.val().min((PWM_ACCURACY.val() * 2) - temp) as u32
                * brightness as u32
                / PWM_ACCURACY.val() as u32;
            ufmt::uwriteln!(
                &mut serial,
                "Brightness: {}\tTemperature: {}\tRed: {}\tGreen: {}",
                brightness,
                temp,
                red,
                green
            )
            .void_unwrap();
            timer1
                .ocr1a
                .write(|w| unsafe { w.bits(if powered { red as u16 } else { 0 }) });
            timer1
                .ocr1b
                .write(|w| unsafe { w.bits(if powered { green as u16 } else { 0 }) });
        } else {
            avr_device::interrupt::free(|cs| {
                let rotary_pins = unsafe { &*(&*ROTARY_PINS.borrow(cs).as_ptr()).as_ptr() };
                if rotary_pins[2].is_low() && !prev_button_state {
                    last_down = millis();
                    prev_button_state = true;
                } else if rotary_pins[2].is_high() && prev_button_state {
                    last_up = millis();
                    let mut diff = last_up - last_down;
                    if last_up < last_down {
                        diff += u32::MAX;
                    }
                    if diff < 200 {
                        ufmt::uwriteln!(&mut serial, "Bumped!").void_unwrap();

                        ROTARY_CHANGE.store(true, Ordering::SeqCst);
                        powered = !powered;
                    }
                    prev_button_state = false;
                }
            });
        }
        delay_ms(50);
    }
}

#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn PCINT2() {
    avr_device::interrupt::free(|cs| {
        let rotary_pins = unsafe { &*(&*ROTARY_PINS.borrow(cs).as_ptr()).as_ptr() };
        if rotary_pins[2].is_low() {
            // Change Temperature when held down
            if rotary_pins[0].is_low() {
                ROTARY_CHANGE.store(true, Ordering::SeqCst);
                let temp_cell = TEMP.borrow(cs);
                let temp = temp_cell.get();
                if rotary_pins[1].is_high() {
                    temp_cell.set((PWM_ACCURACY.val() * 2).min(temp + TEMP_STEP));
                } else if rotary_pins[1].is_low() {
                    temp_cell.set(if temp > TEMP_STEP {
                        temp - TEMP_STEP
                    } else {
                        0
                    });
                }
            }
        } else {
            // Change brightness
            if rotary_pins[0].is_low() {
                ROTARY_CHANGE.store(true, Ordering::SeqCst);
                let brightness_cell = BRIGHTNESS.borrow(cs);
                let brightness = brightness_cell.get();
                if rotary_pins[1].is_high() {
                    brightness_cell.set((PWM_ACCURACY.val()).min(brightness + BRIGHTNESS_STEP));
                } else if rotary_pins[1].is_low() {
                    brightness_cell.set(if brightness > BRIGHTNESS_STEP {
                        brightness - BRIGHTNESS_STEP
                    } else {
                        1
                    });
                }
            }
        }
    });
}

fn get_from_mutex<T: Copy>(mutex: &Mutex<Cell<T>>) -> T {
    avr_device::interrupt::free(|cs| mutex.borrow(cs).get())
}

fn changed(var: &AtomicBool) -> bool {
    avr_device::interrupt::free(|_cs| {
        if var.load(Ordering::SeqCst) {
            var.store(false, Ordering::SeqCst);
            true
        } else {
            false
        }
    })
}

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
