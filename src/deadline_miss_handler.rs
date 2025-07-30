use crate::time::Mono;
use rtic_monotonics::{
    Monotonic, 
    fugit::ExtU32
};

const INIT_TIME: u32 = 0;

// SHARED RESOURCE FOR HANDLING DEADLINE MISS
pub struct DeadlineMissHandlerObject {
    name: &'static str,
    cancelled: bool,
}

impl DeadlineMissHandlerObject {
    pub fn new(name: &'static str) -> Self {
        return DeadlineMissHandlerObject {
            name,
            cancelled: false, 
        };
    }

    pub fn deadline_miss_detected(&mut self) {
        if !self.cancelled {
            defmt::error!(
                "Deadline miss detected for task '{}'",
                self.name
            );
        } else {
            defmt::info!(
                "Deadline miss handler for task '{}' was cancelled",
                self.name
            );
        }
    }

    pub fn cancel_deadline_handler(&mut self) {
        self.cancelled = true;
    }
}

// DEADLINE MISS HANDLER TASK
pub async fn deadline_miss_handler_task(
    deadline_miss_handler_object: &mut impl rtic::Mutex<T = DeadlineMissHandlerObject>,
    period: &mut u32,
) {
    Mono::delay_until(Mono::now() + period.millis()).await;

    deadline_miss_handler_object.lock(|handler_object| {
        handler_object.deadline_miss_detected();
    });
}