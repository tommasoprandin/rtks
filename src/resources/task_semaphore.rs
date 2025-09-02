use crate::time::{Instant, Mono};
use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};
use rtic_monotonics::Monotonic;
use rtic_sync::signal::{Signal, SignalReader, SignalWriter};

pub struct TaskSemaphore;

static mut TASK_SEMAPHORE: MaybeUninit<Signal<()>> = MaybeUninit::uninit();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl TaskSemaphore {
    // The hint is safe since the implementation never leaks the reference out and its used atomically
    #[allow(static_mut_refs)]
    pub fn init(
        activation_watchdog: SignalWriter<'static, Instant>,
    ) -> (TaskSemaphoreWaiter<'static>, TaskSemaphoreSignaler<'static>) {
        let (writer, reader) = if INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            // SAFETY: The CAS operation guarantees at most one initialization even with competing threads, hence if we reach this branch we
            // are guaranteed to be the only initializers of the static signal, and splitting is safe.
            unsafe { TASK_SEMAPHORE.write(Signal::new()).split() }
        } else {
            defmt::panic!("Multiple TaskSemaphore initialization");
        };

        (
            TaskSemaphoreWaiter { inner: reader },
            TaskSemaphoreSignaler {
                inner: writer,
                activation_watchdog,
            },
        )
    }
}

pub struct TaskSemaphoreWaiter<'a> {
    inner: SignalReader<'a, ()>,
}

impl<'a> TaskSemaphoreWaiter<'a> {
    pub async fn wait(&mut self) {
        self.inner.wait().await;
    }
}

pub struct TaskSemaphoreSignaler<'a> {
    inner: SignalWriter<'a, ()>,
    activation_watchdog: SignalWriter<'static, Instant>,
}

impl<'a> TaskSemaphoreSignaler<'a> {
    pub fn signal(&mut self) {
        critical_section::with(|_cs| {
            self.inner.write(());
            // Signal activation to the related deadline watchdog
            self.activation_watchdog.write(Mono::now());
        })
    }
}
