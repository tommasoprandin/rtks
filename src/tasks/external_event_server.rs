use crate::{
    activation_manager,
    resources::{activation_log::ActivationLog, event_queue::EventQueueWaiter},
    time::{Mono, Instant},
    deadline::DeadlineObject,
};
use rtic_sync::signal::SignalWriter;
use rtic_monotonics::Monotonic;

pub const DEADLINE: u32 = 100;

pub async fn external_event_server(
    events: &mut EventQueueWaiter<'_>,
    activation_log: &mut impl rtic::Mutex<T = ActivationLog>,
    deadline_writer: &mut SignalWriter<'static, Instant>,
    deadline: &mut impl rtic::Mutex<T = DeadlineObject>,
    activation_count: &mut u32,
) -> ! {
    activation_manager::activation_sporadic().await;
    loop {
        events.wait().await;
        // start deadline
        deadline_writer.write(Mono::now());
        *activation_count += 1;

        activation_log.lock(|al| {
            al.write();
        });

        // Cancel deadline
        deadline.lock( |deadline| {
            deadline.cancel_deadline(*activation_count);
        });
    }
}
