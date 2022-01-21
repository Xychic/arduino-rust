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

static TMR_OVERFLOW: AtomicBool = AtomicBool::new(false);
static ROTARY_CHANGE: AtomicBool = AtomicBool::new(false);
static VAL: Mutex<Cell<u16>> = Mutex::new(Cell::new(0));
static ROTARY_PINS: Mutex<Cell<MaybeUninit<[Pin<Input<PullUp>, Dynamic>; 2]>>> =
    Mutex::new(Cell::new(MaybeUninit::uninit()));

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);

    let rotary_pins = [
        pins.d8.into_pull_up_input().downgrade(),
        pins.d2.into_pull_up_input().downgrade(),
    ];
    avr_device::interrupt::free(|cs| {
        ROTARY_PINS.borrow(cs).set(MaybeUninit::new(rotary_pins));
    });

    let _red_led_pin = pins.d9.into_output();
    let _green_led_pin = pins.d10.into_output();

    let timer1 = peripherals.TC1;
    // timer1
    //     .tccr1a
    //     .write(|w| w.wgm1().bits(0b01).com1a().match_clear());
    // timer1
    //     .tccr1b
    //     .write(|w| w.wgm1().bits(0b01).cs1().prescale_256());

    timer1.tccr1a.write(|w| {
        w.wgm1()
            .bits(0b10)
            .com1a()
            .match_clear()
            .com1b()
            .match_clear()
    });
    timer1
        .tccr1b
        .write(|w| w.wgm1().bits(0b00).cs1().prescale_256());

    peripherals.EXINT.pcicr.write(|w| unsafe { w.bits(0b100) });
    peripherals.EXINT.pcmsk2.write(|w| unsafe { w.bits(0b100) });

    unsafe {
        avr_device::interrupt::enable();
    }

    ufmt::uwriteln!(&mut serial, "Finished Setup!").void_unwrap();
    loop {
        // if changed(&TMR_OVERFLOW) {
        //     ufmt::uwriteln!(&mut serial, "Timer!").void_unwrap();
        // }
        if changed(&ROTARY_CHANGE) {
            let val = get_from_mutex(&VAL);
            ufmt::uwriteln!(&mut serial, "Val: {}", val).void_unwrap();
            timer1.ocr1a.write(|w| unsafe { w.bits(val) });
            timer1.ocr1b.write(|w| unsafe { w.bits(511 - val) });
        }
        delay_ms(50);
    }
}

#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn PCINT2() {
    avr_device::interrupt::free(|cs| {
        let rotary_pins = unsafe { &*(&*ROTARY_PINS.borrow(cs).as_ptr()).as_ptr() };
        if rotary_pins[1].is_low() {
            ROTARY_CHANGE.store(true, Ordering::SeqCst);
            let val_cell = VAL.borrow(cs);
            let val = val_cell.get();
            if rotary_pins[0].is_low() && val < 511 {
                val_cell.set(val + 1);
            } else if rotary_pins[0].is_high() && val > 0 {
                val_cell.set(val - 1);
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
