use core::sync::atomic::{AtomicU8, Ordering};

use rtic_sync::signal::{Signal, SignalReader, SignalWriter};

pub type EventType = ();
pub struct EventQueue {}

static mut EVENT_QUEUE: Option<Signal<EventType>> = None;
static INITIALIZED: AtomicU8 = AtomicU8::new(0);

impl EventQueue {
    #[allow(static_mut_refs)]
    pub fn new() -> (EventQueueWaiter<'static>, EventQueueSignaler<'static>) {
        let (sender, receiver) = unsafe {
            // SAFETY: The CAS operation ensure mutual exclusive single initialization, the static internal channel is not accessible from the outside directly.
            // We want at most once and atomic semantics
            match INITIALIZED.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => {
                    if EVENT_QUEUE.is_none() {
                        EVENT_QUEUE = Some(Signal::new());
                    }
                    EVENT_QUEUE.as_ref().unwrap().split()
                }
                Err(_) => {
                    defmt::panic!(
                        "EventQueue::new() called multiple times - only one initialization allowed"
                    );
                }
            }
        };
        (
            EventQueueWaiter { inner: receiver },
            EventQueueSignaler { inner: sender },
        )
    }
}

pub struct EventQueueWaiter<'a> {
    inner: SignalReader<'a, EventType>,
}

impl<'a> EventQueueWaiter<'a> {
    pub async fn wait(&mut self) {
        self.inner.wait().await;
    }
}

pub struct EventQueueSignaler<'a> {
    inner: SignalWriter<'a, EventType>,
}

impl<'a> EventQueueSignaler<'a> {
    pub fn signal(&mut self, evt: EventType) {
        self.inner.write(evt);
    }
}
