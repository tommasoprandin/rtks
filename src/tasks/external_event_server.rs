#[cfg(feature = "profiling-external_event_server")]
use crate::profiling::Profiler;
#[cfg(feature = "profiling-external_event_server")]
use crate::profiling::external_event_server::ExternalEventServerProfiler;
use crate::{
    activation_manager,
    deadline::DeadlineProtectedObject,
    resources::{activation_log::ActivationLog, event_queue::EventQueueWaiter},
    time::{Instant, Mono},
};
use rtic_monotonics::Monotonic;
use rtic_sync::signal::SignalWriter;
#[cfg(feature = "profiling-external_event_server")]
use stm32f4xx_hal::dwt::StopWatch;

pub const DEADLINE: u32 = 100;

pub struct ExternalEventServerLocals {
    event_queue: EventQueueWaiter<'static>,
    deadline_activation_writer: SignalWriter<'static, Instant>,
    activation_count: u32,
    #[cfg(feature = "profiling-external_event_server")]
    profiler: ExternalEventServerProfiler,
}

impl ExternalEventServerLocals {
    #[cfg(feature = "profiling-external_event_server")]
    pub fn new(
        event_queue: EventQueueWaiter<'static>,
        deadline_activation_writer: SignalWriter<'static, Instant>,
        stopwatch: StopWatch<'static>,
    ) -> Self {
        Self {
            event_queue,
            deadline_activation_writer,
            activation_count: 0,
            profiler: ExternalEventServerProfiler::new(stopwatch),
        }
    }

    #[cfg(not(feature = "profiling-external_event_server"))]
    pub fn new(
        event_queue: EventQueueWaiter<'static>,
        deadline_activation_writer: SignalWriter<'static, Instant>,
    ) -> Self {
        Self {
            event_queue,
            deadline_activation_writer,
            activation_count: 0,
        }
    }
}

pub struct ExternalEventServerShared<AL, DPO>
where
    AL: rtic::Mutex<T = ActivationLog>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
{
    activation_log: AL,
    deadline_protected_object: DPO,
}

impl<AL, DPO> ExternalEventServerShared<AL, DPO>
where
    AL: rtic::Mutex<T = ActivationLog>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
{
    pub fn new(activation_log: AL, deadline_protected_object: DPO) -> Self {
        Self {
            activation_log,
            deadline_protected_object,
        }
    }
}

pub async fn external_event_server<
    AL: rtic::Mutex<T = ActivationLog>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
>(
    locals: &mut ExternalEventServerLocals,
    shared: &mut ExternalEventServerShared<AL, DPO>,
) -> ! {
    activation_manager::activation_sporadic().await;
    loop {
        locals.event_queue.wait().await;

        // Signal activation to the deadline watchdog
        locals.deadline_activation_writer.write(Mono::now());
        locals.activation_count += 1;

        #[cfg(feature = "profiling-external_event_server")]
        locals.profiler.reset();

        shared.activation_log.lock(|al| {
            al.write();
        });

        #[cfg(feature = "profiling-external_event_server")]
        {
            use crate::profiling::WCET_THRESHOLD;

            locals.profiler.lap();
            locals.profiler.update_wcet();
            if locals.activation_count == WCET_THRESHOLD {
                locals.profiler.log();
            }
        }

        // Cancel deadline
        shared.deadline_protected_object.lock(|dpo| {
            dpo.cancel_deadline(locals.activation_count);
        });
    }
}
