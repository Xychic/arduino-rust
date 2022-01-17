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
    mem,
    sync::atomic::{compiler_fence, AtomicBool, Ordering},
};

static TMR_OVERFLOW: AtomicBool = AtomicBool::new(false);
static ROTARY_CHANGE: AtomicBool = AtomicBool::new(false);

static mut ROTARY_PINS: mem::MaybeUninit<[Pin<Input<PullUp>, Dynamic>; 2]> =
    mem::MaybeUninit::uninit();
static mut VAL: u16 = 0;

#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn TIMER1_COMPA() {
    TMR_OVERFLOW.store(true, Ordering::SeqCst);
}

#[avr_device::interrupt(atmega328p)]
unsafe fn PCINT2() {
    ROTARY_CHANGE.store(true, Ordering::SeqCst);
    let rotary_pins = &*ROTARY_PINS.as_ptr();
    if rotary_pins[1].is_low() {
        // Detect Rising edge
        if rotary_pins[0].is_low() && VAL < u16::MAX {
            VAL += 1;
        } else if rotary_pins[0].is_high() && VAL > 0 {
            VAL -= 1;
        }
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);

    unsafe {
        compiler_fence(Ordering::SeqCst);
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
        compiler_fence(Ordering::SeqCst);
        avr_device::interrupt::enable();
    }

    ufmt::uwriteln!(&mut serial, "Finished Setup!").void_unwrap();
    loop {
        if changed(&TMR_OVERFLOW) {
            ufmt::uwriteln!(&mut serial, "Timer!").void_unwrap();
            if changed(&ROTARY_CHANGE) {
                ufmt::uwriteln!(&mut serial, "Val: {}", unsafe { VAL }).void_unwrap();
            }
        }
        delay_ms(50);
    }
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
