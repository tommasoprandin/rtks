use crate::time::{Mono, Instant};
use rtic_monotonics::{
    Monotonic, 
    fugit::ExtU32
};
use rtic_sync::signal::SignalReader;

// SHARED RESOURCE FOR HANDLING DEADLINE
pub struct DeadlineProtectedObject {
    name: &'static str,
    cancelled: bool,
    misses: u32,
    activations: u32,
}

impl DeadlineProtectedObject {
    pub fn new(
    name: &'static str,
    ) -> Self {
        return DeadlineProtectedObject {
            name,
            cancelled: false, 
            misses: 0,
            activations: 1
        };
    }

    pub fn deadline_miss_detected(&mut self) {
        if !self.cancelled {
            self.misses += 1;
            defmt::error!(
                "Deadline miss detected for task '{}'. Misses: {}, Activations: {}",
                self.name,
                self.misses,
                self.activations
            );
        } else {
            defmt::info!(
                "Deadline for task '{}' was cancelled",
                self.name
            );
        }
        // reset deadline object
        self.cancelled = false;
        self.activations += 1;
    }

    pub fn cancel_deadline(&mut self, activation_id: u32) {
        if activation_id == self.activations {
            self.cancelled = true;
        } 
        // else ignore it, too late to cancel
    }
}

// DEADLINE MISS HANDLERS TASKS
// Periodic
pub struct PeriodicDeadlineWatchdogLocals {
    next_deadline: Option<Instant>,
    period: u32,
}

impl PeriodicDeadlineWatchdogLocals {
    pub fn new(
        next_deadline: Option<Instant>,
        period: u32,
    ) -> Self {
        return PeriodicDeadlineWatchdogLocals {
            next_deadline,
            period,
        };
    }
}
pub async fn periodic_deadline_watchdog(
    locals: &mut PeriodicDeadlineWatchdogLocals,
    deadline_protected_object: &mut impl rtic::Mutex<T = DeadlineProtectedObject>,
) -> ! {
    // Watchdog control loop
    loop {
        if let Some(deadline) = locals.next_deadline {
            Mono::delay_until(deadline).await;
        } else {
            defmt::error!("PeriodicDeadlineWatchdog: next_deadline is None");
            loop {
                Mono::delay_until(Mono::now() + u32::MAX.millis()).await;
            }
        }

        deadline_protected_object.lock(|dpo| {
            dpo.deadline_miss_detected();
        });

        if let Some(deadline) = locals.next_deadline {
            locals.next_deadline = Some(deadline + locals.period.millis());
        }
    }
}

// Sporadic
pub struct SporadicDeadlineWatchdogLocals {
    activation_reader: SignalReader<'static, Instant>,
    next_deadline: Option<Instant>,
    deadline: u32,
}

impl SporadicDeadlineWatchdogLocals {
    pub fn new(
        activation_reader: SignalReader<'static, Instant>,
        next_deadline: Option<Instant>,
        deadline: u32,
    ) -> Self {
        return SporadicDeadlineWatchdogLocals {
            activation_reader,
            next_deadline,
            deadline,
        };
    }
}

pub async fn sporadic_deadline_watchdog(
    locals: &mut SporadicDeadlineWatchdogLocals,
    deadline_protected_object: &mut impl rtic::Mutex<T = DeadlineProtectedObject>,
) -> ! {
    // Watchdog control loop
    loop {
        locals.next_deadline = Some(locals.activation_reader.wait().await + locals.deadline.millis());
        
        if let Some(deadline) = locals.next_deadline {
            Mono::delay_until(deadline).await;
        }

        deadline_protected_object.lock(|dpo| {
            dpo.deadline_miss_detected();
        });
    }
}