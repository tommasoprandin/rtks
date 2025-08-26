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
#[cfg(feature = "rtt")]
use defmt_rtt as _;
#[cfg(feature = "semihosting")]
use defmt_semihosting as _;
#[cfg(not(any(feature = "rtt", feature = "semihosting")))]
compile_error!("No global logger selected, enable either the rtt or semihosting feature");

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
    dispatchers = [EXTI0, EXTI1, EXTI2, EXTI3, EXTI4, EXTI9_5 ])]
mod app {
    use crate::{
        activation_manager,
        deadline::{
            DeadlineProtectedObject, PeriodicDeadlineWatchdogLocals,
            SporadicDeadlineWatchdogLocals, periodic_deadline_watchdog, sporadic_deadline_watchdog,
        },
        resources::{
            activation_log::ActivationLog,
            event_queue::{EventQueue, EventQueueSignaler},
            request_buffer::RequestBuffer,
            task_semaphore::TaskSemaphore,
        },
        tasks::{
            self,
            activation_log_reader::{ActivationLogReaderLocals, ActivationLogReaderShared},
            external_event_server::{ExternalEventServerLocals, ExternalEventServerShared},
            on_call_producer_task::{OnCallProducerLocals, OnCallProducerShared},
            regular_producer_task::{RegularProducerLocals, RegularProducerShared},
        },
        time::{Instant, Mono},
    };
    #[cfg(any(
        feature = "profiling-activation_log_reader",
        feature = "profiling-external_event_server",
        feature = "profiling-on_call_producer",
        feature = "profiling-regular_producer",
    ))]
    use core::mem::MaybeUninit;
    use rtic_monotonics::{fugit::RateExtU32 as _, systick::prelude::*};
    use rtic_sync::make_signal;
    #[cfg(any(
        feature = "profiling-activation_log_reader",
        feature = "profiling-external_event_server",
        feature = "profiling-on_call_producer",
        feature = "profiling-regular_producer",
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
        external_event_server_locals: ExternalEventServerLocals,
        // Activation_Log_Reader
        activation_log_reader_locals: ActivationLogReaderLocals,
        // On_Call_Producer
        on_call_producer_locals: OnCallProducerLocals,
        // Regular_Producer
        regular_producer_locals: RegularProducerLocals,
        // Activation_Log_Reader_Deadline_Miss_Handler
        activation_log_reader_watchdog_locals: SporadicDeadlineWatchdogLocals,
        // External_Event_Server_Deadline_Miss_Handler
        external_event_server_watchdog_locals: SporadicDeadlineWatchdogLocals,
        // On_Call_Producer_Deadline_Miss_Handler
        on_call_producer_watchdog_locals: SporadicDeadlineWatchdogLocals,
        // Regular_Producer_Deadline_Miss_Handler
        regular_producer_watchdog_locals: PeriodicDeadlineWatchdogLocals,
    }

    #[init(local = [
        #[cfg(any(
            feature = "profiling-activation_log_reader",
            feature = "profiling-external_event_server",
            feature = "profiling-on_call_producer",
            feature = "profiling-regular_producer",
        ))]
        dwt_storage: MaybeUninit<Dwt> = MaybeUninit::uninit(),
        #[cfg(feature = "profiling-activation_log_reader")]
        activation_log_reader_times: [u32; 6] = [0; 6],
        #[cfg(feature = "profiling-external_event_server")]
        external_event_server_times: [u32; 4] = [0; 4],
        #[cfg(feature = "profiling-on_call_producer")]
        on_call_producer_times: [u32; 6] = [0; 6],
        #[cfg(feature = "profiling-regular_producer")]
        regular_producer_times: [u32; 8] = [0; 8],
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
        ))]
        let dwt = unsafe {
            let dwt = core.DWT.constrain(core.DCB, &clocks);
            cx.local.dwt_storage.write(dwt);
            cx.local.dwt_storage.assume_init_ref()
        };

        // Setup monotonic timer
        Mono::start(core.SYST, clocks.sysclk().to_Hz());
        // Setup ACTIVATION_INSTANT
        activation_manager::set_activation_time();

        // Setup activation log
        let activation_log = ActivationLog::new();
        // Setup barrier for on call producer
        let (barrier_writer, barrier_reader) = make_signal!(());

        // Setup activation log reader deadline
        let activation_log_reader_deadline_protected_object =
            DeadlineProtectedObject::new("Activation_Log_Reader");
        let (activation_log_reader_activation_writer, activation_log_reader_activation_reader) =
            make_signal!(Instant);
        // Setup activation log reader semaphore
        let (activation_log_reader_waiter, activation_log_reader_signaler) =
            TaskSemaphore::init(activation_log_reader_activation_writer);

        // Setup external event server deadline
        let external_event_server_deadline_protected_object =
            DeadlineProtectedObject::new("External_Event_Server");
        let (external_event_server_activation_writer, external_event_server_activation_reader) =
            make_signal!(Instant);
        // Setup event queue
        let (event_waiter, event_signaler) =
            EventQueue::init(external_event_server_activation_writer);

        // Setup on call producer deadline
        let on_call_producer_deadline_protected_object =
            DeadlineProtectedObject::new("On_Call_Producer");
        let (on_call_producer_activation_writer, on_call_producer_activation_reader) =
            make_signal!(Instant);
        // Setup request buffer
        let request_buffer = RequestBuffer::new(barrier_writer, on_call_producer_activation_writer);

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
                // Activation_Log_Reader
                activation_log_reader_locals: ActivationLogReaderLocals::new(
                    activation_log_reader_waiter,
                    #[cfg(feature = "profiling-activation_log_reader")]
                    dwt.stopwatch(cx.local.activation_log_reader_times),
                ),
                // External_Event_Server
                external_event_server_locals: ExternalEventServerLocals::new(
                    event_waiter,
                    #[cfg(feature = "profiling-external_event_server")]
                    dwt.stopwatch(cx.local.external_event_server_times),
                ),
                // On_Call_Producer
                on_call_producer_locals: OnCallProducerLocals::new(
                    barrier_reader,
                    #[cfg(feature = "profiling-on_call_producer")]
                    dwt.stopwatch(cx.local.on_call_producer_times),
                ),
                // Regular_Producer
                regular_producer_locals: RegularProducerLocals::new(
                    activation_log_reader_signaler,
                    #[cfg(feature = "profiling-regular_producer")]
                    dwt.stopwatch(cx.local.regular_producer_times),
                ),
                // Activation_Log_Reader_Deadline_Miss_Handler
                activation_log_reader_watchdog_locals: SporadicDeadlineWatchdogLocals::new(
                    activation_log_reader_activation_reader,
                    None,
                    tasks::activation_log_reader::DEADLINE,
                ),
                // External_Event_Server_Deadline_Miss_Handler
                external_event_server_watchdog_locals: SporadicDeadlineWatchdogLocals::new(
                    external_event_server_activation_reader,
                    None,
                    tasks::external_event_server::DEADLINE,
                ),
                // On_Call_Producer_Deadline_Miss_Handler
                on_call_producer_watchdog_locals: SporadicDeadlineWatchdogLocals::new(
                    on_call_producer_activation_reader,
                    None,
                    tasks::on_call_producer_task::DEADLINE,
                ),
                // Regular_Producer_Deadline_Miss_Handler
                regular_producer_watchdog_locals: PeriodicDeadlineWatchdogLocals::new(
                    Some(
                        activation_manager::get_activation_instant()
                            + tasks::regular_producer_task::DEADLINE.millis(),
                    ),
                    tasks::regular_producer_task::PERIOD,
                ),
            },
        )
    }

    #[task(priority = 3, local=[activation_log_reader_locals], shared=[activation_log, activation_log_reader_deadline_protected_object])]
    async fn activation_log_reader(cx: activation_log_reader::Context) -> ! {
        tasks::activation_log_reader::activation_log_reader(
            cx.local.activation_log_reader_locals,
            &mut ActivationLogReaderShared::new(
                cx.shared.activation_log,
                cx.shared.activation_log_reader_deadline_protected_object,
            ),
        )
        .await;
    }

    #[task(priority = 16, local=[external_event_server_locals], shared=[activation_log, external_event_server_deadline_protected_object])]
    async fn external_event_server(cx: external_event_server::Context) -> ! {
        tasks::external_event_server::external_event_server(
            cx.local.external_event_server_locals,
            &mut ExternalEventServerShared::new(
                cx.shared.activation_log,
                cx.shared.external_event_server_deadline_protected_object,
            ),
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
            ),
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

    #[task(priority = 15, local = [activation_log_reader_watchdog_locals], shared =[activation_log_reader_deadline_protected_object])]
    async fn activation_log_reader_deadline_miss_handler(
        mut cx: activation_log_reader_deadline_miss_handler::Context,
    ) -> ! {
        sporadic_deadline_watchdog(
            cx.local.activation_log_reader_watchdog_locals,
            &mut cx.shared.activation_log_reader_deadline_protected_object,
        )
        .await;
    }

    #[task(priority = 15, local = [external_event_server_watchdog_locals], shared =[external_event_server_deadline_protected_object])]
    async fn external_event_server_deadline_miss_handler(
        mut cx: external_event_server_deadline_miss_handler::Context,
    ) -> ! {
        sporadic_deadline_watchdog(
            cx.local.external_event_server_watchdog_locals,
            &mut cx.shared.external_event_server_deadline_protected_object,
        )
        .await;
    }

    #[task(priority = 15, local = [on_call_producer_watchdog_locals], shared =[on_call_producer_deadline_protected_object])]
    async fn on_call_producer_deadline_miss_handler(
        mut cx: on_call_producer_deadline_miss_handler::Context,
    ) -> ! {
        sporadic_deadline_watchdog(
            cx.local.on_call_producer_watchdog_locals,
            &mut cx.shared.on_call_producer_deadline_protected_object,
        )
        .await;
    }

    #[task(priority = 15, local = [regular_producer_watchdog_locals], shared =[regular_producer_deadline_protected_object])]
    async fn regular_producer_deadline_miss_handler(
        mut cx: regular_producer_deadline_miss_handler::Context,
    ) -> ! {
        periodic_deadline_watchdog(
            cx.local.regular_producer_watchdog_locals,
            &mut cx.shared.regular_producer_deadline_protected_object,
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
            let next_time = Mono::now() + 5.secs();
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
