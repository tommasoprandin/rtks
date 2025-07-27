#![no_std]
#![no_main]

mod auxiliary;
mod resources;
mod tasks;
mod time;
mod workload;

use cortex_m::interrupt;
use cortex_m_semihosting::debug::{self, EXIT_FAILURE};
use defmt_semihosting as _;
use stm32f4xx_hal as _;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    interrupt::disable();

    defmt::error!("Panic: {}", info);
    debug::exit(EXIT_FAILURE);

    loop {}
}

#[rtic::app(
    device = stm32f4xx_hal::pac,
    dispatchers = [EXTI0, EXTI1, EXTI2]
)]
mod app {

    use crate::{
        resources::{
            activation_log::ActivationLog,
            event_queue::{EventQueue, EventQueueSignaler, EventQueueWaiter},
            task_semaphore::{TaskSemaphore, TaskSemaphoreSignaler, TaskSemaphoreWaiter},
        },
        tasks,
        time::Mono,
    };
    use cortex_m::asm::nop;
    use rtic_monotonics::{fugit::RateExtU32 as _, systick::prelude::*};
    use stm32f4xx_hal::rcc::RccExt;

    // Shared resources go here
    #[shared]
    struct Shared {
        activation_log: ActivationLog,
    }

    // Local resources go here
    #[local]
    struct Local {
        event_signaler: EventQueueSignaler<'static>,
        event_waiter: EventQueueWaiter<'static>,
        activation_log_reader_signaler: TaskSemaphoreSignaler<'static>,
        activation_log_reader_waiter: TaskSemaphoreWaiter<'static>,
    }

    // Timer struct
    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        defmt::info!("Init");

        // Extract device from context
        let peripherals = ctx.device;
        let core = ctx.core;

        // Clocks setup
        let rcc = peripherals.RCC.constrain();
        let clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(168.MHz())
            .pclk1(42.MHz())
            .freeze();

        defmt::info!("Clocks initialized");

        // Setup monotonic timer
        Mono::start(core.SYST, clocks.sysclk().to_Hz());

        // Setup event queue
        let (event_waiter, event_signaler) = EventQueue::new();
        // Setup activation log
        let activation_log = ActivationLog::new();
        // Setup activation log reader semaphore
        let (activation_log_reader_waiter, activation_log_reader_signaler) = TaskSemaphore::new();

        external_event_server::spawn().expect("Error spawning external event server");
        producer_task::spawn().expect("Error spawning producer task");
        activation_log_reader::spawn().expect("Error spawning activatio log reader task");

        (
            Shared {
                // Initialization of shared resources go here
                activation_log,
            },
            Local {
                // Initialization of local resources go here
                event_signaler,
                event_waiter,
                activation_log_reader_signaler,
                activation_log_reader_waiter,
            },
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            nop();
        }
    }

    #[task(priority = 11, local=[event_waiter], shared=[activation_log])]
    async fn external_event_server(mut cx: external_event_server::Context) -> ! {
        tasks::external_event_server::external_event_server(
            cx.local.event_waiter,
            &mut cx.shared.activation_log,
        )
        .await;
    }

    #[task(priority = 3, local=[activation_log_reader_waiter], shared=[activation_log])]
    async fn activation_log_reader(mut cx: activation_log_reader::Context) -> ! {
        tasks::activation_log_reader::activation_log_reader(
            cx.local.activation_log_reader_waiter,
            &mut cx.shared.activation_log,
        ).await;
    }

    #[task(priority = 1, local=[event_signaler, activation_log_reader_signaler])]
    async fn producer_task(cx: producer_task::Context) -> ! {
        let events = cx.local.event_signaler;
        let al_semaphore = cx.local.activation_log_reader_signaler;
        loop {
            for _ in 1..=3 {
                Mono::delay(500.millis()).await;
                events.signal(());
            }
            Mono::delay(1_000.millis()).await;
            al_semaphore.signal();
        }
    }
}
