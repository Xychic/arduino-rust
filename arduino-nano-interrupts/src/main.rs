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
use panic_halt as _;

use core::{
    cell, mem,
    sync::atomic::{AtomicBool, Ordering},
};

static TMR_OVERFLOW: AtomicBool = AtomicBool::new(false);
static ROTARY_CHANGE: AtomicBool = AtomicBool::new(false);
static VAL: avr_device::interrupt::Mutex<cell::Cell<u16>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

static mut ROTARY_PINS: mem::MaybeUninit<[Pin<Input<PullUp>, Dynamic>; 2]> =
    mem::MaybeUninit::uninit();

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);

    unsafe {
        ROTARY_PINS = mem::MaybeUninit::new([
            pins.d8.into_pull_up_input().downgrade(),
            pins.d2.into_pull_up_input().downgrade(),
        ]);
    }

    let timer1 = peripherals.TC1;
    timer1.tccr1a.write(|w| unsafe { w.bits(0) });
    timer1.tccr1b.write(|w| w.cs1().prescale_256());
    timer1.ocr1a.write(|w| unsafe { w.bits(62499) });
    timer1.tcnt1.write(|w| unsafe { w.bits(0) });
    // Enable the timer interrupt
    timer1.timsk1.write(|w| w.ocie1a().set_bit());

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
            ufmt::uwriteln!(&mut serial, "Val: {}", get_from_mutex(&VAL)).void_unwrap();
        }
        delay_ms(50);
    }
}

#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn TIMER1_COMPA() {
    TMR_OVERFLOW.store(true, Ordering::SeqCst);
}

#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
unsafe fn PCINT2() {
    let rotary_pins = &*ROTARY_PINS.as_ptr();
    // Detect Rising edge
    if rotary_pins[1].is_low() {
        // Store the fact that a change has been detected
        ROTARY_CHANGE.store(true, Ordering::SeqCst);
        avr_device::interrupt::free(|cs| {
            // Get the current value from the atomic
            let val_cell = VAL.borrow(cs);
            let val = val_cell.get();
            if rotary_pins[0].is_low() && val < u16::MAX {
                val_cell.set(val + 1);
            } else if rotary_pins[0].is_high() && val > 0 {
                val_cell.set(val - 1);
            }
        });
    }
}

fn get_from_mutex<T>(mutex: &avr_device::interrupt::Mutex<cell::Cell<T>>) -> T
where
    T: Copy,
{
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
