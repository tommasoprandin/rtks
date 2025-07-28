use crate::{
    resources::{
        activation_log::ActivationLog,
        task_semaphore::TaskSemaphoreWaiter,
    },
    production_workload,
};

pub async fn activation_log_reader(
    semaphore: &mut TaskSemaphoreWaiter<'_>,
    activation_log: &mut impl rtic::Mutex<T = ActivationLog>,
) -> ! {
    loop {
        semaphore.wait().await;
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
        })
    }
}
