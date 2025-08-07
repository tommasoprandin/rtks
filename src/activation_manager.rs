use crate::{
    time::{Mono, Instant},
};
use rtic_monotonics::{Monotonic, fugit::ExtU32};

const RELATIVE_OFFSET: u32 = 1_000;

pub fn activation_time() -> Instant {
    Mono::now() + RELATIVE_OFFSET.millis()
}

pub async fn activation_sporadic() {
    Mono::delay_until(activation_time()).await;
}

pub async fn activation_cyclic() {
    Mono::delay_until(activation_time()).await;
}