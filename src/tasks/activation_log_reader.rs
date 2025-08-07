use crate::{
    resources::{
        activation_log::ActivationLog,
        task_semaphore::TaskSemaphoreWaiter,
    },
    production_workload,
    deadline::DeadlineObject,
    time::{Mono, Instant},
};
use rtic_sync::signal::SignalWriter;
use rtic_monotonics::Monotonic;

pub const DEADLINE: u32 = 1_000;

pub async fn activation_log_reader(
    semaphore: &mut TaskSemaphoreWaiter<'_>,
    activation_log: &mut impl rtic::Mutex<T = ActivationLog>,
    deadline_writer: &mut SignalWriter<'static, Instant>,
    deadline: &mut impl rtic::Mutex<T = DeadlineObject>,
    activation_count: &mut u32,
) -> ! {
    loop {
        semaphore.wait().await;
        // start deadline
        deadline_writer.write(Mono::now());
        *activation_count += 1;

        if let Err(err) = production_workload::small_whetstone(1_000) {
            defmt::error!(
                "Error computing whetstone in activation log reader: {}",
                err
            );
        }
        activation_log.lock(|al| {
            let (activations, last) = al.read();
            defmt::info!(
                "Activation log reader: activations = {}, last = {}",
                activations,
                last
            );
        });

        // Cancel deadline
        deadline.lock( |deadline| {
            deadline.cancel_deadline(*activation_count);
        });
    }
}
