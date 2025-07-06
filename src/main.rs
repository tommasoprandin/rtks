#![no_std]
#![no_main]

use cortex_m as _;
use defmt_semihosting as _;
use panic_halt as _;

use stm32f4xx_hal as _;

#[rtic::app(
    device = stm32f4xx_hal::pac,
    dispatchers = [EXTI0, EXTI1]
)]
mod app {

    use cortex_m_semihosting::debug;

    // Shared resources go here
    #[shared]
    struct Shared {}

    // Local resources go here
    #[local]
    struct Local {}

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        defmt::info!("Init");

        // TODO setup monotonic if used
        // let sysclk = { /* clock setup + returning sysclk as an u32 */ };
        // let token = rtic_monotonics::create_systick_token!();
        // rtic_monotonics::systick::Systick::new(cx.core.SYST, sysclk, token);

        task1::spawn().ok();

        (
            Shared {
                // Initialization of shared resources go here
            },
            Local {
                // Initialization of local resources go here
            },
        )
    }

    // Optional idle, can be removed if not needed.
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        defmt::info!("Idle alive");
        for i in 0..10 {
            defmt::info!("Idle loop tick {}", i);
        }
        defmt::info!("Goodbye");
        debug::exit(debug::EXIT_SUCCESS);
        loop {}
    }

    // TODO: Add tasks
    #[task(priority = 1)]
    async fn task1(_cx: task1::Context) {
        defmt::info!("Hello from task1!");
    }
}
