use crate::time::{Mono, Instant};
use rtic_monotonics::{Monotonic, fugit::ExtU32};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

const RELATIVE_OFFSET: u32 = 1_000;

static ACTIVATION_FLAG: AtomicBool = AtomicBool::new(false);
static ACTIVATION_TICKS: AtomicU32 = AtomicU32::new(0);

pub fn set_activation_time() {
    if ACTIVATION_FLAG
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        ACTIVATION_TICKS.store(
            (Mono::now() + RELATIVE_OFFSET.millis())
            .duration_since_epoch()
            .ticks() as u32,
            Ordering::SeqCst);
    } else {
        panic!("set_activation_instant: already set");
    }
}

pub fn get_activation_instant() -> Instant {
    if !ACTIVATION_FLAG.load(Ordering::SeqCst) {
        panic!("get_activation_instant: not set");
    }
    Instant::from_ticks(ACTIVATION_TICKS.load(Ordering::SeqCst))
}

pub async fn activation_sporadic() {
    Mono::delay_until(get_activation_instant()).await;
}

pub async fn activation_cyclic() {
    Mono::delay_until(get_activation_instant()).await;
}
