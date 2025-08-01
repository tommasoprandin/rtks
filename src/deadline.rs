use crate::time::{Mono, Instant};
use rtic_monotonics::{
    Monotonic, 
    fugit::ExtU32
};

//const ZERO_TIME: u32 = 0;

// SHARED RESOURCE FOR HANDLING DEADLINE
pub struct DeadlineObject {
    name: &'static str,
    cancelled: bool,
    misses: u32,
    activations: u32,
}

impl DeadlineObject {
    pub fn new(
    name: &'static str,
    ) -> Self {
        return DeadlineObject {
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

    pub fn cancel_deadline(&mut self, activation_ID: u32) {
        if activation_ID == self.activations {
            self.cancelled = true;
        } 
        // else ignore it, too late to cancel
    }
}

// DEADLINE MISS HANDLER TASK
pub async fn deadline_watchdog(
    deadline_object: &mut impl rtic::Mutex<T = DeadlineObject>,
    next_deadline: &mut Instant, 
    period: u32,
) -> ! {
    // Watchdog control loop
    loop {
        Mono::delay_until(*next_deadline).await;

        deadline_object.lock(|deadline_object| {
            deadline_object.deadline_miss_detected();
        });

        *next_deadline += period.millis();
    }
}