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
    dispatchers = [EXTI0, EXTI1, EXTI2, EXTI3, EXTI4, EXTI5])]
mod app {

    use cortex_m_semihosting::debug;
    use rtic_sync::{make_signal, signal::{SignalReader}};

    use crate::tasks::on_call_producer_task;
    use crate::resources::request_buffer::RequestBuffer;

    // Shared resources go here
    #[shared]
    struct Shared {
        request_buffer: RequestBuffer,
    }

    // Local resources go here
    #[local]
    struct Local {
        // Regular_Producer
        next_time: fugit::TimerInstantU32<Mono>,
        // On_Call_Producer
        current_workload: u32,
        barrier_reader: SignalReader<'static, ()>,
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        defmt::info!("Init");

        // TODO setup monotonic if used
        // let sysclk = { /* clock setup + returning sysclk as an u32 */ };
        // let token = rtic_monotonics::create_systick_token!();
        // rtic_monotonics::systick::Systick::new(cx.core.SYST, sysclk, token);

        let (barrier_writer, barrier_reader) = make_signal!(());

        regular_producer::spawn().ok();
        on_call_producer::spawn().ok();

        (
            Shared {
                // Initialization of shared resources
                request_buffer: RequestBuffer::new(barrier_writer),
            },
            Local {
                // Initialization of local resources
                next_time: fugit::TimerInstantU32::<Mono>::from_millis(0),
                current_workload: 0,
                barrier_reader,
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

    #[task(priority = 1, local = [next_time], shared = [request_buffer])]
    async fn regular_producer(cx: regular_producer::Context) {
        regular_producer_task::regular_producer_task(cx);
    }

    #[task(priority = 5, local = [current_workload, barrier_reader], shared =[request_buffer])]
    async fn on_call_producer(cx: on_call_producer::Context) {
        on_call_producer_task::on_call_producer_task(cx).await;
    }
}
