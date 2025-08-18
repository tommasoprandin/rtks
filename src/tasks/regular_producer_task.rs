use crate::{
    activation_manager, auxiliary,
    deadline::DeadlineProtectedObject,
    production_workload,
    resources::{request_buffer::RequestBuffer, task_semaphore::TaskSemaphoreSignaler},
    time::{Instant, Mono},
};
use rtic_monotonics::{Monotonic, fugit::ExtU32};
#[cfg(feature = "profiling")]
use stm32f4xx_hal::dwt::Dwt;

pub const PERIOD: u32 = 1_000;
pub const DEADLINE: u32 = 500;

const REGULAR_PRODUCER_WORKLOAD: u32 = 756;
const ON_CALL_PRODUCER_WORKLOAD: u32 = 278;
const ACTIVATION_CONDITION: usize = 2;

pub async fn regular_producer_task(
    next_time: &mut Instant,
    request_buffer: &mut impl rtic::Mutex<T = RequestBuffer>,
    activation_log_reader_signaler: &mut TaskSemaphoreSignaler<'_>,
    deadline_protected_object: &mut impl rtic::Mutex<T = DeadlineProtectedObject>,
    activation_count: &mut u32,
    #[cfg(feature = "profiling")] dwt: &Dwt,
) -> ! {
    #[cfg(feature = "profiling")]
    let mut times: [u32; 3] = [0; 3];
    #[cfg(feature = "profiling")]
    let mut stopwatch = dwt.stopwatch(&mut times);

    activation_manager::activation_cyclic().await;
    loop {
        #[cfg(feature = "profiling")]
        stopwatch.reset();
        *next_time = Mono::now() + PERIOD.millis();
        *activation_count += 1;

        // BEGIN REGULAR_PRODUCER_OPERATION
        // Standard workload
        if let Err(err) = production_workload::small_whetstone(REGULAR_PRODUCER_WORKLOAD) {
            defmt::error!(
                "Error computing whetstone in regular producer operation: {}",
                err
            );
        }
        #[cfg(feature = "profiling")]
        stopwatch.lap(); // Lap 1: Workload execution

        // Helper tasks activations
        if auxiliary::due_activation(ACTIVATION_CONDITION) {
            // on_call_producer activation
            request_buffer.lock(|buffer| {
                if !buffer.deposit(ON_CALL_PRODUCER_WORKLOAD) {
                    defmt::info!("Failed sporadic activation.");
                }
            })
        }
        if auxiliary::check_due() {
            activation_log_reader_signaler.signal();
        }
        defmt::info!("End of cyclic activation.");
        // END REGULAR_PRODUCER_OPERATION

        // Cancel deadline
        deadline_protected_object.lock(|dpo| {
            dpo.cancel_deadline(*activation_count);
        });

        #[cfg(feature = "profiling")]
        stopwatch.lap(); // Lap 2: Total response time

        #[cfg(feature = "profiling")]
        {
            let workload_time = stopwatch.lap_time(1).unwrap().as_micros();
            let overhead_time = stopwatch.lap_time(2).unwrap().as_micros();
            let total_time = workload_time + overhead_time;
            defmt::info!(
                "Regular producer task: workload={}us\ttotal={}us",
                workload_time,
                total_time,
            );
        }

        Mono::delay_until(*next_time).await;
    }
}
