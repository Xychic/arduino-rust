#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::{delay_ms, hal::wdt, prelude::*};
use panic_halt as _;

// static mut VAL: u16 = 0;

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 9600);

    let button1 = pins.d8.into_pull_up_input().downgrade();
    let button2 = pins.d2.into_pull_up_input().downgrade();

    let mut watchdog = wdt::Wdt::new(peripherals.WDT, &peripherals.CPU.mcusr);
    watchdog.start(wdt::Timeout::Ms4000).unwrap(); // Watchdog will restart device if not fed in 4 seconds

    let mut val: u16 = 0;

    unsafe {
        avr_device::interrupt::enable();
    }

    ufmt::uwriteln!(&mut serial, "Finished Setup!").void_unwrap();
    loop {
        if button1.is_low() {
            val += 1;
            ufmt::uwriteln!(&mut serial, "Val: {}", val).void_unwrap();
        } else if button2.is_low() {
            val -= 1;
            ufmt::uwriteln!(&mut serial, "Val: {}", val).void_unwrap();
        }

        watchdog.feed(); // Say hi to the watch dog
        delay_ms(50);
    }
}
