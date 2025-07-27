use crate::resources::{activation_log::ActivationLog, event_queue::EventQueueWaiter};

pub async fn external_event_server(
    events: &mut EventQueueWaiter<'_>,
    activation_log: &mut impl rtic::Mutex<T = ActivationLog>,
) -> ! {
    loop {
        events.wait().await;
        defmt::info!("Executing external event server operation");
        activation_log.lock(|al| {
            al.write();
        })
    }
}
