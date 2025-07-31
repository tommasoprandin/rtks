use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

use rtic_sync::signal::{Signal, SignalReader, SignalWriter};

pub type EventType = ();
pub struct EventQueue;

static mut EVENT_QUEUE: MaybeUninit<Signal<EventType>> = MaybeUninit::uninit();
static INITIALIZED: AtomicBool = AtomicBool::new(false);

impl EventQueue {
    // The hint is safe since the implementation never leaks the reference out and its used atomically
    #[allow(static_mut_refs)]
    pub fn init() -> (EventQueueWaiter<'static>, EventQueueSignaler<'static>) {
        let (writer, reader) = if INITIALIZED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            // SAFETY: The CAS operation guarantees at most one initialization even with competing threads, hence if we reach this branch we
            // are guaranteed to be the only initializers of the static signal, and splitting is safe.
            unsafe { EVENT_QUEUE.write(Signal::new()).split() }
        } else {
            defmt::panic!("Multiple EventQueue initialization");
        };

        (
            EventQueueWaiter { inner: reader },
            EventQueueSignaler { inner: writer },
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

#[derive(Clone)]
pub struct EventQueueSignaler<'a> {
    inner: SignalWriter<'a, EventType>,
}

impl<'a> EventQueueSignaler<'a> {
    pub fn signal(&mut self, evt: EventType) {
        self.inner.write(evt);
    }
}
