#![no_std]
#![no_main]

mod activation_manager;
mod auxiliary;
mod deadline;
mod production_workload;
mod profiling;
mod resources;
mod tasks;
mod time;

use cortex_m::interrupt;
use cortex_m_semihosting::debug::{self, EXIT_FAILURE};
#[cfg(feature = "board")]
use defmt_rtt as _;
#[cfg(feature = "qemu")]
use defmt_semihosting as _;
use stm32f4xx_hal as _;

#[cfg(any(
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-regular_producer",
    feature = "profiling-activation_log",
    feature = "profiling-request_buffer",
))]
pub const WCET_THRESHOLD: u32 = 100;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    interrupt::disable();

    defmt::error!("Panic: {}", info);
    debug::exit(EXIT_FAILURE);

    loop {}
}

#[rtic::app(
    device = stm32f4xx_hal::pac,
    dispatchers = [EXTI0, EXTI1, EXTI2, EXTI3, EXTI4, EXTI9_5])]
mod app {

    use core::mem::MaybeUninit;

    use crate::{
        activation_manager,
        deadline::{
            DeadlineProtectedObject, periodic_deadline_watchdog, sporadic_deadline_watchdog,
        },
        resources::{
            activation_log::ActivationLog,
            event_queue::{EventQueue, EventQueueSignaler, EventQueueWaiter},
            request_buffer::RequestBuffer,
            task_semaphore::{TaskSemaphore, TaskSemaphoreWaiter},
        },
        tasks::{
            self,
            on_call_producer_task::{OnCallProducerLocals, OnCallProducerShared},
            regular_producer_task::{RegularProducerLocals, RegularProducerShared},
        },
        time::{Instant, Mono},
    };
    use rtic_monotonics::{fugit::RateExtU32 as _, systick::prelude::*};
    use rtic_sync::{make_signal, signal::SignalReader, signal::SignalWriter};
    #[cfg(any(
        feature = "profiling-activation_log_reader",
        feature = "profiling-external_event_server",
        feature = "profiling-on_call_producer",
        feature = "profiling-regular_producer",
        feature = "profiling-activation_log",
        feature = "profiling-request_buffer",
    ))]
    use stm32f4xx_hal::dwt::{Dwt, DwtExt};

    use stm32f4xx_hal::{interrupt, pac::NVIC, rcc::RccExt};

    // Shared resources go here
    #[shared]
    struct Shared {
        activation_log: ActivationLog,
        request_buffer: RequestBuffer,

        activation_log_reader_deadline_protected_object: DeadlineProtectedObject,
        external_event_server_deadline_protected_object: DeadlineProtectedObject,
        on_call_producer_deadline_protected_object: DeadlineProtectedObject,
        regular_producer_deadline_protected_object: DeadlineProtectedObject,
    }

    // Local resources go here
    #[local]
    struct Local {
        // Interrupt_Handler
        event_signaler: EventQueueSignaler<'static>,
        // External_Event_Server
        event_waiter: EventQueueWaiter<'static>,
        external_event_server_activation_writer: SignalWriter<'static, Instant>,
        external_event_server_activation_count: u32,
        // Activation_Log_Reader
        activation_log_reader_waiter: TaskSemaphoreWaiter<'static>, 
        activation_log_reader_activation_writer: SignalWriter<'static, Instant>,
        activation_log_reader_activation_count: u32,
        // On_Call_Producer
        on_call_producer_locals: OnCallProducerLocals,
        // Regular_Producer
        regular_producer_locals: RegularProducerLocals,
        // Activation_Log_Reader_Deadline_Miss_Handler
        activation_log_reader_activation_reader: SignalReader<'static, Instant>,
        activation_log_reader_deadline_value: u32,
        activation_log_reader_next_deadline: Instant,
        // External_Event_Server_Deadline_Miss_Handler
        external_event_server_activation_reader: SignalReader<'static, Instant>,
        external_event_server_deadline_value: u32,
        external_event_server_next_deadline: Instant,
        // On_Call_Producer_Deadline_Miss_Handler
        on_call_producer_activation_reader: SignalReader<'static, Instant>,
        on_call_producer_deadline_value: u32,
        on_call_producer_next_deadline: Instant,
        // Regular_Producer_Deadline_Miss_Handler
        regular_producer_period: u32,
        regular_producer_next_deadline: Instant,
    }

    #[init(local = [
        #[cfg(any(
            feature = "profiling-activation_log_reader",
            feature = "profiling-external_event_server",
            feature = "profiling-on_call_producer",
            feature = "profiling-regular_producer",
            feature = "profiling-activation_log",
            feature = "profiling-request_buffer",
        ))]
        dwt_storage: MaybeUninit<Dwt> = MaybeUninit::uninit(),
        #[cfg(feature = "profiling-on_call_producer")]
        on_call_producer_activation_times: [u32; 2] = [0; 2],
        #[cfg(feature = "profiling-regular_producer")]
        regular_producer_times: [u32; 3] = [0; 3],
        activation_log_reader_semaphore: TaskSemaphore = TaskSemaphore::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("Init");

        // Extract device from context
        let peripherals = cx.device;
        let core = cx.core;

        // Clocks setup
        let rcc = peripherals.RCC.constrain();
        let clocks = rcc
            .cfgr
            .use_hse(8.MHz())
            .sysclk(168.MHz())
            .pclk1(42.MHz())
            .freeze();

        #[cfg(any(
            feature = "profiling-external_event_server",
            feature = "profiling-activation_log_reader",
            feature = "profiling-on_call_producer",
            feature = "profiling-regular_producer",
            feature = "profiling-activation_log",
            feature = "profiling-request_buffer",
        ))]
        let dwt = unsafe {
            let dwt = core.DWT.constrain(core.DCB, &clocks);
            cx.local.dwt_storage.write(dwt);
            cx.local.dwt_storage.assume_init_ref()
        };

        // Setup monotonic timer
        Mono::start(core.SYST, clocks.sysclk().to_Hz());

        // Setup event queue
        let (event_waiter, event_signaler) = EventQueue::init();
        // Setup activation log
        let activation_log = ActivationLog::new();
        // Setup activation log reader semaphore
        let (activation_log_reader_waiter, activation_log_reader_signaler) =
            cx.local.activation_log_reader_semaphore.split();
        // Setup barrier for on call producer
        let (barrier_writer, barrier_reader) = make_signal!(());
        // Setup request buffer
        let request_buffer = RequestBuffer::new(barrier_writer);
        // Setup external event server deadline
        let external_event_server_deadline_protected_object =
            DeadlineProtectedObject::new("External_Event_Server");
        let (external_event_server_activation_writer, external_event_server_activation_reader) =
            make_signal!(Instant);
        // Setup activation log reader deadline
        let activation_log_reader_deadline_protected_object =
            DeadlineProtectedObject::new("Activation_Log_Reader");
        let (activation_log_reader_activation_writer, activation_log_reader_activation_reader) =
            make_signal!(Instant);
        // Setup on call producer deadline
        let on_call_producer_deadline_protected_object =
            DeadlineProtectedObject::new("On_Call_Producer");
        let (on_call_producer_activation_writer, on_call_producer_activation_reader) =
            make_signal!(Instant);
        // Setup regular producer deadline
        let regular_producer_deadline_protected_object =
            DeadlineProtectedObject::new("Regular_Producer");

        activation_log_reader_deadline_miss_handler::spawn()
            .expect("Error spawning activation log reader deadline miss handler");
        external_event_server_deadline_miss_handler::spawn()
            .expect("Error spawning external event server deadline miss handler");
        on_call_producer_deadline_miss_handler::spawn()
            .expect("Error spawning on call producer deadline miss handler");
        regular_producer_deadline_miss_handler::spawn()
            .expect("Error spawning regular producer deadline miss handler");

        external_event_server::spawn().expect("Error spawning external event server");
        activation_log_reader::spawn().expect("Error spawning activation log reader task");
        regular_producer::spawn().expect("Error spawning regular producer task");
        on_call_producer::spawn().expect("Error spawning on call producer task");
        interrupt_generator::spawn().expect("Error spawning interrupt generator task");

        (
            Shared {
                // Initialization of shared resources go here
                request_buffer,
                activation_log,
                activation_log_reader_deadline_protected_object,
                external_event_server_deadline_protected_object,
                on_call_producer_deadline_protected_object,
                regular_producer_deadline_protected_object,
            },
            Local {
                // Initialization of local resources go here
                event_signaler,
                // External_Event_Server
                event_waiter,
                external_event_server_activation_writer,
                external_event_server_activation_count: 0,
                // Activation_Log_Reader
                activation_log_reader_waiter,
                activation_log_reader_activation_writer,
                activation_log_reader_activation_count: 0,
                // On_Call_Producer
                on_call_producer_locals: OnCallProducerLocals::new(
                    barrier_reader,
                    on_call_producer_activation_writer,
                    #[cfg(feature = "profiling-on_call_producer")]
                    dwt.stopwatch(cx.local.on_call_producer_activation_times),
                ),
                // Regular_Producer
                regular_producer_locals: RegularProducerLocals::new(
                    activation_log_reader_signaler,
                    #[cfg(feature = "profiling-regular_producer")]
                    dwt.stopwatch(cx.local.regular_producer_times),
                ),
                // Activation_Log_Reader_Deadline_Miss_Handler
                activation_log_reader_activation_reader,
                activation_log_reader_deadline_value: tasks::activation_log_reader::DEADLINE,
                activation_log_reader_next_deadline: Instant::from_ticks(0),
                // External_Event_Server_Deadline_Miss_Handler
                external_event_server_activation_reader,
                external_event_server_deadline_value: tasks::external_event_server::DEADLINE,
                external_event_server_next_deadline: Instant::from_ticks(0),
                // On_Call_Producer_Deadline_Miss_Handler
                on_call_producer_activation_reader,
                on_call_producer_deadline_value: tasks::on_call_producer_task::DEADLINE,
                on_call_producer_next_deadline: Instant::from_ticks(0),
                // Regular_Producer_Deadline_Miss_Handler
                regular_producer_period: tasks::regular_producer_task::PERIOD,
                regular_producer_next_deadline: activation_manager::activation_time()
                    + tasks::regular_producer_task::DEADLINE.millis(),
            },
        )
    }

    #[task(priority = 3, local=[activation_log_reader_waiter, activation_log_reader_activation_writer, activation_log_reader_activation_count], shared=[activation_log, activation_log_reader_deadline_protected_object])]
    async fn activation_log_reader(mut cx: activation_log_reader::Context) -> ! {
        tasks::activation_log_reader::activation_log_reader(
            cx.local.activation_log_reader_waiter,
            &mut cx.shared.activation_log,
            &mut cx.local.activation_log_reader_activation_writer,
            &mut cx.shared.activation_log_reader_deadline_protected_object,
            &mut cx.local.activation_log_reader_activation_count,
        )
        .await;
    }

    #[task(priority = 11, local=[event_waiter, external_event_server_activation_writer, external_event_server_activation_count], shared=[activation_log, external_event_server_deadline_protected_object])]
    async fn external_event_server(mut cx: external_event_server::Context) -> ! {
        tasks::external_event_server::external_event_server(
            cx.local.event_waiter,
            &mut cx.shared.activation_log,
            &mut cx.local.external_event_server_activation_writer,
            &mut cx.shared.external_event_server_deadline_protected_object,
            &mut cx.local.external_event_server_activation_count,
        )
        .await;
    }

    #[task(priority = 5, local = [on_call_producer_locals], shared = [request_buffer, on_call_producer_deadline_protected_object])]
    async fn on_call_producer(cx: on_call_producer::Context) {
        tasks::on_call_producer_task::on_call_producer_task(
            cx.local.on_call_producer_locals,
            &mut OnCallProducerShared::new(
                cx.shared.request_buffer,
                cx.shared.on_call_producer_deadline_protected_object,
            )    
        )
        .await;
    }

    #[task(priority = 7, local = [regular_producer_locals], shared = [request_buffer, regular_producer_deadline_protected_object])]
    async fn regular_producer(cx: regular_producer::Context) {
        tasks::regular_producer_task::regular_producer_task(
            cx.local.regular_producer_locals,
            &mut RegularProducerShared::new(
                cx.shared.request_buffer,
                cx.shared.regular_producer_deadline_protected_object,
            ),
        )
        .await;
    }

    #[task(priority = 12, local = [activation_log_reader_activation_reader, activation_log_reader_next_deadline, activation_log_reader_deadline_value], shared =[activation_log_reader_deadline_protected_object])]
    async fn activation_log_reader_deadline_miss_handler(
        mut cx: activation_log_reader_deadline_miss_handler::Context,
    ) -> ! {
        sporadic_deadline_watchdog(
            &mut cx.shared.activation_log_reader_deadline_protected_object,
            &mut cx.local.activation_log_reader_activation_reader,
            &mut cx.local.activation_log_reader_next_deadline,
            *cx.local.activation_log_reader_deadline_value,
        )
        .await;
    }

    #[task(priority = 12, local = [external_event_server_activation_reader, external_event_server_next_deadline, external_event_server_deadline_value], shared =[external_event_server_deadline_protected_object])]
    async fn external_event_server_deadline_miss_handler(
        mut cx: external_event_server_deadline_miss_handler::Context,
    ) -> ! {
        sporadic_deadline_watchdog(
            &mut cx.shared.external_event_server_deadline_protected_object,
            &mut cx.local.external_event_server_activation_reader,
            &mut cx.local.external_event_server_next_deadline,
            *cx.local.external_event_server_deadline_value,
        )
        .await;
    }

    #[task(priority = 12, local = [on_call_producer_activation_reader, on_call_producer_next_deadline, on_call_producer_deadline_value], shared =[on_call_producer_deadline_protected_object])]
    async fn on_call_producer_deadline_miss_handler(
        mut cx: on_call_producer_deadline_miss_handler::Context,
    ) -> ! {
        sporadic_deadline_watchdog(
            &mut cx.shared.on_call_producer_deadline_protected_object,
            &mut cx.local.on_call_producer_activation_reader,
            &mut cx.local.on_call_producer_next_deadline,
            *cx.local.on_call_producer_deadline_value,
        )
        .await;
    }

    #[task(priority = 12, local = [regular_producer_next_deadline, regular_producer_period], shared =[regular_producer_deadline_protected_object])]
    async fn regular_producer_deadline_miss_handler(
        mut cx: regular_producer_deadline_miss_handler::Context,
    ) -> ! {
        periodic_deadline_watchdog(
            &mut cx.shared.regular_producer_deadline_protected_object,
            &mut cx.local.regular_producer_next_deadline,
            *cx.local.regular_producer_period,
        )
        .await;
    }

    #[task(priority = 13)]
    async fn interrupt_generator(_cx: interrupt_generator::Context) -> ! {
        unsafe {
            NVIC::unmask(interrupt::USART1);
        }
        activation_manager::activation_cyclic().await;
        loop {
            let next_time = Mono::now() + 10.secs();
            NVIC::pend(interrupt::USART1);
            defmt::info!("USART1 interrupt generated");
            Mono::delay_until(next_time).await;
        }
    }

    #[task(binds = USART1, local = [event_signaler])]
    fn interrupt_handler(cx: interrupt_handler::Context) {
        cx.local.event_signaler.signal(());
    }
}
