#![no_std]
#![no_main]

use cortex_m as _;
use cortex_m_semihosting::debug;
use defmt_semihosting as _;
use panic_halt as _;

use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    for i in 0..10 {
        defmt::info!("Hello, world! {}", i);
    }
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
