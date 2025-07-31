use crate::{
    auxiliary,
    deadline::DeadlineStopper,
    production_workload,
    resources::{request_buffer::RequestBuffer, task_semaphore::TaskSemaphoreSignaler},
    time::Mono,
};
use rtic_monotonics::{Monotonic, fugit::ExtU32};

type Instant = <Mono as Monotonic>::Instant;

pub const PERIOD: u32 = 1000;

const REGULAR_PRODUCER_WORKLOAD: u32 = 756;
const ON_CALL_PRODUCER_WORKLOAD: u32 = 278;
const ACTIVATION_CONDITION: usize = 2;

pub async fn regular_producer_task(
    next_time: &mut Instant,
    request_buffer: &mut impl rtic::Mutex<T = RequestBuffer>,
    activation_log_reader_signaler: &mut TaskSemaphoreSignaler<'_>,
    deadline_stopper: &mut DeadlineStopper<'_>,
) -> ! {
    loop {
        *next_time = Mono::now() + PERIOD.millis();
        defmt::info!("Start of cyclic activation");

        // BEGIN REGULAR_PRODUCER_OPERATION
        if let Err(err) = production_workload::small_whetstone(REGULAR_PRODUCER_WORKLOAD) {
            defmt::error!(
                "Error computing whetstone in regular producer operation: {}",
                err
            );
        }
        if auxiliary::due_activation(ACTIVATION_CONDITION) {
            request_buffer.lock(|buffer| {
                if !buffer.deposit(ON_CALL_PRODUCER_WORKLOAD) {
                    defmt::info!("Failed sporadic activation.");
                }
            })
        }
        if auxiliary::check_due() {
            activation_log_reader_signaler.signal();
        }
        // Mono::delay(1000.millis()).await;
        defmt::info!("End of cyclic activation.");
        // END REGULAR_PRODUCER_OPERATION

        // Cancel deadline miss handler
        deadline_stopper.done();

        Mono::delay_until(*next_time).await;
    }
}
