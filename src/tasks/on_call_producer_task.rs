#[cfg(feature = "profiling-on_call_producer")]
use crate::profiling::{Profiler as _, on_call_producer::OnCallProducerProfiler};
use crate::{
    activation_manager, deadline::DeadlineProtectedObject, production_workload,
    resources::request_buffer::RequestBuffer,
};
use rtic_sync::signal::SignalReader;
#[cfg(feature = "profiling-on_call_producer")]
use stm32f4xx_hal::dwt::StopWatch;

pub const DEADLINE: u32 = 800;

pub struct OnCallProducerLocals {
    current_workload: u32,
    barrier_reader: SignalReader<'static, ()>,
    activation_count: u32,
    #[cfg(feature = "profiling-on_call_producer")]
    profiler: OnCallProducerProfiler,
}

impl OnCallProducerLocals {
    #[cfg(feature = "profiling-on_call_producer")]
    pub fn new(barrier_reader: SignalReader<'static, ()>, stopwatch: StopWatch<'static>) -> Self {
        Self {
            current_workload: 0,
            barrier_reader,
            activation_count: 0,
            profiler: OnCallProducerProfiler::new(stopwatch),
        }
    }

    #[cfg(not(feature = "profiling-on_call_producer"))]
    pub fn new(barrier_reader: SignalReader<'static, ()>) -> Self {
        Self {
            current_workload: 0,
            barrier_reader,
            activation_count: 0,
        }
    }
}

pub struct OnCallProducerShared<RB, DPO>
where
    RB: rtic::Mutex<T = RequestBuffer>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
{
    request_buffer: RB,
    deadline_protected_object: DPO,
}

impl<RB, DPO> OnCallProducerShared<RB, DPO>
where
    RB: rtic::Mutex<T = RequestBuffer>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
{
    pub fn new(request_buffer: RB, deadline_protected_object: DPO) -> Self {
        Self {
            request_buffer,
            deadline_protected_object,
        }
    }
}

pub async fn on_call_producer_task<
    RB: rtic::Mutex<T = RequestBuffer>,
    DPO: rtic::Mutex<T = DeadlineProtectedObject>,
>(
    locals: &mut OnCallProducerLocals,
    shared: &mut OnCallProducerShared<RB, DPO>,
) -> ! {
    activation_manager::activation_sporadic().await;
    loop {
        locals.barrier_reader.wait().await;
        locals.activation_count += 1;

        #[cfg(feature = "profiling-on_call_producer")]
        locals.profiler.reset();

        shared.request_buffer.lock(|buffer| {
            #[cfg(feature = "profiling-on_call_producer")]
            locals.profiler.lap(); // Lap 1: start rb_extract
            locals.current_workload = buffer.extract();
            #[cfg(feature = "profiling-on_call_producer")]
            locals.profiler.lap(); // Lap 2: end rb_extract
        });
        #[cfg(feature = "profiling-on_call_producer")]
        locals.profiler.lap(); // Lap 3: extract workload enclosing

        // BEGIN ON_CALL_PRODUCER_OPERATION
        if let Err(err) = production_workload::small_whetstone(locals.current_workload) {
            defmt::error!(
                "Error computing whetstone in on call producer operation: {}",
                err
            );
        }
        #[cfg(feature = "profiling-on_call_producer")]
        locals.profiler.lap(); // Lap 4: ocp_small_whetstone 

        defmt::info!("End of sporadic activation.");
        #[cfg(feature = "profiling-on_call_producer")]
        locals.profiler.lap(); // Lap 5: put_line
        // END ON_CALL_PRODUCER_OPERATION

        // Cancel deadline
        shared.deadline_protected_object.lock(|dpo| {
            dpo.cancel_deadline(locals.activation_count);
        });

        #[cfg(feature = "profiling-on_call_producer")]
        {
            use crate::profiling::WCET_THRESHOLD;

            locals.profiler.update_wcet();
            if locals.activation_count == WCET_THRESHOLD {
                locals.profiler.log();
                defmt::panic!("On call producer profiling finished");
            }
        }
    }
}
