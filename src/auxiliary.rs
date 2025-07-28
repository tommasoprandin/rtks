#![allow(unused)]
use core::sync::atomic::{AtomicUsize, Ordering};

const DUE_ACTIVATION_MOD: usize = 5;
const CHECK_DUE_MOD: usize = 1000;
const CHECK_DUE_FACTOR: usize = 3;

pub fn due_activation(param: usize) -> bool {
    static REQUEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

    counter % DUE_ACTIVATION_MOD == param % DUE_ACTIVATION_MOD
}

pub fn check_due() -> bool {
    static RUN_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let counter = RUN_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

    (counter % CHECK_DUE_MOD) % CHECK_DUE_FACTOR == 0
}
