#![no_std]
#![no_main]

use cortex_m as _;
use defmt_semihosting as _;
use panic_halt as _;

use stm32f4xx_hal as _;

mod tasks;
mod resources;

#[rtic::app(
    device = stm32f4xx_hal::pac,
    dispatchers = [EXTI0, EXTI1, EXTI2],
)]
mod app {

    use cortex_m_semihosting::debug;
    use rtic_sync::channel::{Receiver, Sender};

    use crate::tasks::on_call_producer_task;
    use crate::resources::request_buffer::RequestBuffer;

    const CAPACITY: usize = 5;
    // Shared resources go here
    #[shared]
    struct Shared {
        val: u32,

        request_buffer: RequestBuffer,
    }

    // Local resources go here
    #[local]
    struct Local {
        s: Sender<'static, u32, CAPACITY>,
        r: Receiver<'static, u32, CAPACITY>,

        // On_Call_Producer
        current_workload: u32,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        defmt::info!("Init");

        // TODO setup monotonic if used
        // let sysclk = { /* clock setup + returning sysclk as an u32 */ };
        // let token = rtic_monotonics::create_systick_token!();
        // rtic_monotonics::systick::Systick::new(cx.core.SYST, sysclk, token);

        let (s, r) = rtic_sync::make_channel!(u32, CAPACITY);

        task1::spawn().ok();
        task2::spawn().ok();
        on_call_producer::spawn().ok();

        (
            Shared {
                // Initialization of shared resources go here
                val: 0,
                request_buffer: RequestBuffer::new(),
            },
            Local {
                // Initialization of local resources go here
                s,
                r,
                current_workload: 0,
            },
        )
    }

    // Optional idle, can be removed if not needed.
    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        defmt::info!("Idling...");
        defmt::info!("Goodbye");
        debug::exit(debug::EXIT_SUCCESS);
        loop {}
    }

    #[task(priority = 1, local=[s], shared = [val])]
    async fn task1(cx: task1::Context) {
        let mut val = cx.shared.val;
        let s = cx.local.s;

        defmt::info!("Hello from task1!");

        val.lock(|v| {
            *v = 1;
            defmt::info!("Shared value is now: {}", v);
        });
        

        match s.send(42).await {
            Ok(_) => defmt::info!("Sent value 42"),
            Err(e) => defmt::error!("Failed to send value: {}", e),
        }; 
    }

    #[task(priority = 2, local = [r], shared = [val])]
    async fn task2(cx: task2::Context) {
        let mut val = cx.shared.val;
        let r = cx.local.r;

        defmt::info!("Hello from task2!");

        val.lock(|v| {
            *v = 2;
            defmt::info!("Shared value is now: {}", v);
        });

        match r.recv().await {
            Ok(value) => defmt::info!("Received value: {}", value),
            Err(e) => defmt::error!("Failed to receive value: {}", e),
        };
    }

    #[task(priority = 3, local = [current_workload], shared =[request_buffer])]
    async fn on_call_producer(cx: on_call_producer::Context) {
        on_call_producer_task::on_call_producer_task(cx);
    }
}
