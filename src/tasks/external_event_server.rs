#[cfg(feature = "profiling-external_event_server")]
use crate::profiling::Profiler;
#[cfg(feature = "profiling-external_event_server")]
use crate::profiling::external_event_server::ExternalEventServerProfiler;
use crate::{
    activation_manager,
    deadline::DeadlineProtectedObject,
    resources::{activation_log::ActivationLog, event_queue::EventQueueWaiter},
};
#[cfg(feature = "profiling-external_event_server")]
use stm32f4xx_hal::dwt::StopWatch;

pub const DEADLINE: u32 = 100;

pub struct ExternalEventServerLocals {
    event_queue: EventQueueWaiter<'static>,
    activation_count: u32,
    #[cfg(feature = "profiling-external_event_server")]
    profiler: ExternalEventServerProfiler,
}

impl ExternalEventServerLocals {
    #[cfg(feature = "profiling-external_event_server")]
    pub fn new(event_queue: EventQueueWaiter<'static>, stopwatch: StopWatch<'static>) -> Self {
        Self {
            event_queue,
            activation_count: 0,
            profiler: ExternalEventServerProfiler::new(stopwatch),
        }
    }

    #[cfg(not(feature = "profiling-external_event_server"))]
    pub fn new(event_queue: EventQueueWaiter<'static>) -> Self {
        Self {
            event_queue,
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
        locals.activation_count += 1;

        #[cfg(feature = "profiling-external_event_server")]
        locals.profiler.reset();

        shared.activation_log.lock(|al| {
            #[cfg(feature = "profiling-external_event_server")]
            locals.profiler.lap(); // Lap 1: start al_write
            al.write();
            #[cfg(feature = "profiling-external_event_server")]
            locals.profiler.lap(); // Lap 2: end al_write
        });

        #[cfg(feature = "profiling-external_event_server")]
        {
            use crate::profiling::WCET_THRESHOLD;

            locals.profiler.lap(); // Lap 3: ees_write
            locals.profiler.update_wcet();
            if locals.activation_count == WCET_THRESHOLD {
                locals.profiler.log();
                defmt::panic!("External event server profiling finished");
            }
        }

        // Cancel deadline
        shared.deadline_protected_object.lock(|dpo| {
            dpo.cancel_deadline(locals.activation_count);
        });
    }
}
