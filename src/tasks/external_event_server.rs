use crate::{
    activation_manager,
    resources::{activation_log::ActivationLog, event_queue::EventQueueWaiter},
    time::{Mono, Instant},
    deadline::DeadlineProtectedObject,
};
use rtic_sync::signal::SignalWriter;
use rtic_monotonics::Monotonic;

pub const DEADLINE: u32 = 100;

pub async fn external_event_server(
    events: &mut EventQueueWaiter<'_>,
    activation_log: &mut impl rtic::Mutex<T = ActivationLog>,
    activation_writer: &mut SignalWriter<'static, Instant>,
    deadline_protected_object: &mut impl rtic::Mutex<T = DeadlineProtectedObject>,
    activation_count: &mut u32,
) -> ! {
    activation_manager::activation_sporadic().await;
    loop {
        events.wait().await;

        // Signal activation to the deadline watchdog
        activation_writer.write(Mono::now());
        *activation_count += 1;

        activation_log.lock(|al| {
            al.write();
        });

        // Cancel deadline
        deadline_protected_object.lock( |dpo| {
            dpo.cancel_deadline(*activation_count);
        });
    }
}
