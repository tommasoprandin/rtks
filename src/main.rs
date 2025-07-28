#![no_std]
#![no_main]

mod auxiliary;
mod resources;
mod tasks;
mod time;
mod production_workload;

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
    dispatchers = [EXTI0, EXTI1, EXTI2, EXTI3, EXTI4])]
mod app {

    use crate::{
        resources::{
            activation_log::ActivationLog,
            event_queue::{EventQueue, EventQueueSignaler, EventQueueWaiter},
            request_buffer::RequestBuffer,
            task_semaphore::{TaskSemaphore, TaskSemaphoreSignaler, TaskSemaphoreWaiter},
        },
        tasks,
        time::Mono,
    };
    use cortex_m::asm::nop;
    use rtic_monotonics::{fugit::RateExtU32 as _, systick::prelude::*};
    use rtic_sync::{make_signal, signal::SignalReader};
    use stm32f4xx_hal::rcc::RccExt;

    type Instant = <Mono as Monotonic>::Instant;

    // Shared resources go here
    #[shared]
    struct Shared {
        activation_log: ActivationLog,
        request_buffer: RequestBuffer,
    }

    // Local resources go here
    #[local]
    struct Local {
        event_signaler: EventQueueSignaler<'static>,
        event_waiter: EventQueueWaiter<'static>,
        activation_log_reader_signaler: TaskSemaphoreSignaler<'static>,
        activation_log_reader_waiter: TaskSemaphoreWaiter<'static>,
        // Regular_Producer
        next_time: Instant,
        // On_Call_Producer
        current_workload: u32,
        barrier_reader: SignalReader<'static, ()>,
    }

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
        // Setup barrier for on call producer
        let (barrier_writer, barrier_reader) = make_signal!(());
        // Setup request buffer
        let request_buffer = RequestBuffer::new(barrier_writer);

        external_event_server::spawn().expect("Error spawning external event server");
        activation_log_reader::spawn().expect("Error spawning activation log reader task");
        regular_producer::spawn().expect("Error spawning regular producer task");
        on_call_producer::spawn().expect("Error spawning on call producer task");

        (
            Shared {
                // Initialization of shared resources go here
                request_buffer,
                activation_log,
            },
            Local {
                // Initialization of local resources go here
                event_signaler,
                event_waiter,
                activation_log_reader_signaler,
                activation_log_reader_waiter,
                next_time: Mono::now(),
                current_workload: 0,
                barrier_reader,
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
        )
        .await;
    }

    #[task(priority = 7, local = [next_time, activation_log_reader_signaler], shared = [request_buffer])]
    async fn regular_producer(mut cx: regular_producer::Context) {
        tasks::regular_producer_task::regular_producer_task(
            cx.local.next_time,
            &mut cx.shared.request_buffer,
            cx.local.activation_log_reader_signaler,
        )
        .await;
    }

    #[task(priority = 5, local = [current_workload, barrier_reader], shared =[request_buffer])]
    async fn on_call_producer(mut cx: on_call_producer::Context) {
        tasks::on_call_producer_task::on_call_producer_task(
            &mut cx.shared.request_buffer,
            cx.local.current_workload,
            cx.local.barrier_reader,
        )
        .await;
    }
}
