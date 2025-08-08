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
pub async fn periodic_deadline_watchdog(
    deadline_protected_object: &mut impl rtic::Mutex<T = DeadlineProtectedObject>,
    next_deadline: &mut Instant, 
    period: u32,
) -> ! {
    // Watchdog control loop
    loop {
        Mono::delay_until(*next_deadline).await;

        deadline_protected_object.lock(|dpo| {
            dpo.deadline_miss_detected();
        });

        *next_deadline += period.millis();
    }
}

pub async fn sporadic_deadline_watchdog(
    deadline_protected_object: &mut impl rtic::Mutex<T = DeadlineProtectedObject>,
    activation_reader: &mut SignalReader<'static, Instant>,
    next_deadline: &mut Instant, 
    deadline: u32,
) -> ! {
    // Watchdog control loop
    loop {
        *next_deadline = activation_reader.wait().await + deadline.millis();
        
        Mono::delay_until(*next_deadline).await;

        deadline_protected_object.lock(|dpo| {
            dpo.deadline_miss_detected();
        });
    }
}