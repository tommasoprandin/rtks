use core::sync::atomic::{AtomicU8, Ordering};
use rtic_sync::signal::{Signal, SignalReader, SignalWriter};
pub struct TaskSemaphore {}

static mut SEMAPHORE: Option<Signal<()>> = None;
static INITIALIZED: AtomicU8 = AtomicU8::new(0);

impl TaskSemaphore {
    #[allow(static_mut_refs)]
    pub fn new() -> (TaskSemaphoreWaiter<'static>, TaskSemaphoreSignaler<'static>) {
        let (sender, receiver) = unsafe {
            // SAFETY: The CAS operation ensure mutual exclusive single initialization, the static internal channel is not accessible from the outside directly.
            // We want at most once and atomic semantics
            match INITIALIZED.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => {
                    if SEMAPHORE.is_none() {
                        SEMAPHORE = Some(Signal::new());
                    }
                    SEMAPHORE.as_ref().unwrap().split()
                }
                Err(_) => {
                    defmt::panic!(
                        "EventQueue::new() called multiple times - only one initialization allowed"
                    );
                }
            }
        };
        (
            TaskSemaphoreWaiter { inner: receiver },
            TaskSemaphoreSignaler { inner: sender },
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
}

impl<'a> TaskSemaphoreSignaler<'a> {
    pub fn signal(&mut self) {
        self.inner.write(());
    }
}
