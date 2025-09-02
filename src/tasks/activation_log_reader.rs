#[cfg(feature = "profiling-activation_log_reader")]
use crate::profiling::{Profiler as _, activation_log_reader::ActivationLogReaderProfiler};
use crate::{
    activation_manager,
    deadline::DeadlineProtectedObject,
    production_workload,
    resources::{activation_log::ActivationLog, task_semaphore::TaskSemaphoreWaiter},
};
#[cfg(feature = "profiling-activation_log_reader")]
use stm32f4xx_hal::dwt::StopWatch;

pub const DEADLINE: u32 = 1_000;

pub struct ActivationLogReaderLocals {
    semaphore: TaskSemaphoreWaiter<'static>,
    activation_count: u32,
    #[cfg(feature = "profiling-activation_log_reader")]
    profiler: ActivationLogReaderProfiler,
}

impl ActivationLogReaderLocals {
    #[cfg(feature = "profiling-activation_log_reader")]
    pub fn new(semaphore: TaskSemaphoreWaiter<'static>, stopwatch: StopWatch<'static>) -> Self {
        Self {
            semaphore,
            activation_count: 0,
            profiler: ActivationLogReaderProfiler::new(stopwatch),
        }
    }

    #[cfg(not(feature = "profiling-activation_log_reader"))]
    pub fn new(semaphore: TaskSemaphoreWaiter<'static>) -> Self {
        Self {
            semaphore,
            activation_count: 0,
        }
    }
}

pub struct ActivationLogReaderShared<AL, DPO>
where
    AL: rtic::Mutex<T = ActivationLog>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
{
    activation_log: AL,
    deadline_protected_object: DPO,
}

impl<AL, DPO> ActivationLogReaderShared<AL, DPO>
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

pub async fn activation_log_reader<
    AL: rtic::Mutex<T = ActivationLog>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
>(
    locals: &mut ActivationLogReaderLocals,
    shared: &mut ActivationLogReaderShared<AL, DPO>,
) -> ! {
    activation_manager::activation_sporadic().await;
    loop {
        locals.semaphore.wait().await;
        locals.activation_count += 1;

        #[cfg(feature = "profiling-activation_log_reader")]
        locals.profiler.reset();

        if let Err(err) = production_workload::small_whetstone(1_000) {
            defmt::error!(
                "Error computing whetstone in activation log reader: {}",
                err
            );
        }
        #[cfg(feature = "profiling-activation_log_reader")]
        locals.profiler.lap(); // Lap 1: alr_small_whetstone

        shared.activation_log.lock(|al| {
            #[cfg(feature = "profiling-activation_log_reader")]
            locals.profiler.lap(); // Lap 2: start al_read

            let (_activations, _last) = al.read();

            #[cfg(feature = "profiling-activation_log_reader")]
            locals.profiler.lap(); // Lap 3: end al_read
        });
        #[cfg(feature = "profiling-activation_log_reader")]
        locals.profiler.lap(); // Lap 4: alr_read

        defmt::info!("End of parameterless sporadic activation.");
        #[cfg(feature = "profiling-activation_log_reader")]
        locals.profiler.lap(); // Lap 5: put_line

        // Cancel deadline
        shared.deadline_protected_object.lock(|dpo| {
            #[cfg(feature = "profiling-activation_log_reader")]
            locals.profiler.lap(); // Lap 6: start cancel_deadline
            dpo.cancel_deadline(locals.activation_count);
            #[cfg(feature = "profiling-activation_log_reader")]
            locals.profiler.lap(); // Lap 7: end cancel_deadline
        });
        #[cfg(feature = "profiling-activation_log_reader")]
        locals.profiler.lap(); // Lap 8: dpo_cancel_deadline

        #[cfg(feature = "profiling-activation_log_reader")]
        {
            use crate::profiling::WCET_THRESHOLD;

            locals.profiler.update_wcet();
            if locals.activation_count == WCET_THRESHOLD {
                locals.profiler.log();
                defmt::panic!("Activation log reader finished profiling");
            }
        }
    }
}
