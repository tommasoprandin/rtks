use rtic_monotonics::Monotonic;

use crate::time::Mono;

const ACTIVATION_MOD: u32 = 100;

type Instant = <Mono as rtic_monotonics::Monotonic>::Instant;

pub struct ActivationLog {
    activation_counter: u32,
    last_activation_time: Option<Instant>,
}

impl ActivationLog {
    pub fn new() -> Self {
        ActivationLog {
            activation_counter: 0,
            last_activation_time: None,
        }
    }

    pub fn write(&mut self) {
        self.activation_counter = (self.activation_counter + 1) % ACTIVATION_MOD;
        self.last_activation_time = Some(Mono::now());
    }

    pub fn read(&self) -> (u32, Option<Instant>) {
        (self.activation_counter, self.last_activation_time)
    }
}
