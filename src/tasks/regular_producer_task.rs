#[cfg(feature = "profiling-regular_producer")]
use crate::profiling::{Profiler as _, regular_producer::RegularProducerProfiler};
use crate::{
    activation_manager, auxiliary,
    deadline::DeadlineProtectedObject,
    production_workload,
    resources::{request_buffer::RequestBuffer, task_semaphore::TaskSemaphoreSignaler},
    time::{Instant, Mono},
};
use rtic_monotonics::{Monotonic, fugit::ExtU32};
#[cfg(feature = "profiling-regular_producer")]
use stm32f4xx_hal::dwt::StopWatch;

pub const PERIOD: u32 = 1_000;
pub const DEADLINE: u32 = 500;

const REGULAR_PRODUCER_WORKLOAD: u32 = 756;
const ON_CALL_PRODUCER_WORKLOAD: u32 = 278;
const ACTIVATION_CONDITION: usize = 2;

pub struct RegularProducerLocals {
    activation_log_reader_signaler: TaskSemaphoreSignaler<'static>,
    next_time: Instant,
    activation_count: u32,
    #[cfg(feature = "profiling-regular_producer")]
    profiler: RegularProducerProfiler,
}

impl RegularProducerLocals {
    #[cfg(feature = "profiling-regular_producer")]
    pub fn new(
        alr_signaler: TaskSemaphoreSignaler<'static>,
        stopwatch: StopWatch<'static>,
    ) -> Self {
        Self {
            activation_log_reader_signaler: alr_signaler,
            next_time: Mono::now(),
            activation_count: 0,
            profiler: RegularProducerProfiler::new(stopwatch),
        }
    }

    #[cfg(not(feature = "profiling-regular_producer"))]
    pub fn new(alr_signaler: TaskSemaphoreSignaler<'static>) -> Self {
        Self {
            activation_log_reader_signaler: alr_signaler,
            next_time: Mono::now(),
            activation_count: 0,
        }
    }
}

pub struct RegularProducerShared<RB, DPO>
where
    RB: rtic::Mutex<T = RequestBuffer>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
{
    request_buffer: RB,
    deadline_protected_object: DPO,
}

impl<RB, DPO> RegularProducerShared<RB, DPO>
where
    RB: rtic::Mutex<T = RequestBuffer>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
{
    pub fn new(request_buffer: RB, deadline_protected_object: DPO) -> Self {
        Self {
            request_buffer,
            deadline_protected_object,
        }
    }
}

pub async fn regular_producer_task<
    RB: rtic::Mutex<T = RequestBuffer>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
>(
    locals: &mut RegularProducerLocals,
    shared: &mut RegularProducerShared<RB, DPO>,
) -> ! {
    activation_manager::activation_cyclic().await;
    loop {
        #[cfg(feature = "profiling-regular_producer")]
        locals.profiler.reset();

        locals.next_time = Mono::now() + PERIOD.millis();
        locals.activation_count += 1;

        // BEGIN REGULAR_PRODUCER_OPERATION
        // Standard workload
        if let Err(err) = production_workload::small_whetstone(REGULAR_PRODUCER_WORKLOAD) {
            defmt::error!(
                "Error computing whetstone in regular producer operation: {}",
                err
            );
        }

        #[cfg(feature = "profiling-regular_producer")]
        locals.profiler.lap(); // Lap 1: Workload execution

        // Helper tasks activations
        if auxiliary::due_activation(ACTIVATION_CONDITION) {
            // on_call_producer activation
            shared.request_buffer.lock(|buffer| {
                if !buffer.deposit(ON_CALL_PRODUCER_WORKLOAD) {
                    defmt::info!("Failed sporadic activation.");
                }
            })
        }
        if auxiliary::check_due() {
            locals.activation_log_reader_signaler.signal();
        }
        defmt::info!("End of cyclic activation.");
        // END REGULAR_PRODUCER_OPERATION

        // Cancel deadline
        shared.deadline_protected_object.lock(|dpo| {
            dpo.cancel_deadline(locals.activation_count);
        });

        #[cfg(feature = "profiling-regular_producer")]
        {
            use crate::profiling::WCET_THRESHOLD;

            locals.profiler.lap(); // Lap 2: Total response time
            locals.profiler.update_wcet();
            if locals.activation_count == WCET_THRESHOLD {
                locals.profiler.log();
            }
        }

        Mono::delay_until(locals.next_time).await;
    }
}
