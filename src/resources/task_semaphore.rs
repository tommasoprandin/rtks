use rtic_sync::signal::{Signal, SignalReader, SignalWriter};
pub struct TaskSemaphore {
    inner: Signal<()>,
}

impl TaskSemaphore {
    pub const fn new() -> Self {
        Self {
            inner: Signal::new(),
        }
    }

    pub fn split(&self) -> (TaskSemaphoreWaiter<'_>, TaskSemaphoreSignaler<'_>) {
        let (writer, reader) = self.inner.split();
        (
            TaskSemaphoreWaiter { inner: reader },
            TaskSemaphoreSignaler { inner: writer },
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
