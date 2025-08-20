#[cfg(feature = "profiling-on_call_producer")]
use crate::profiling::{Profiler as _, on_call_producer::OnCallProducerProfiler};
use crate::{
    activation_manager,
    deadline::DeadlineProtectedObject,
    production_workload,
    resources::request_buffer::RequestBuffer,
    time::{Instant, Mono},
};
use rtic_monotonics::Monotonic;
use rtic_sync::signal::{SignalReader, SignalWriter};
#[cfg(feature = "profiling-on_call_producer")]
use stm32f4xx_hal::dwt::StopWatch;

pub const DEADLINE: u32 = 800;

pub struct OnCallProducerLocals {
    current_workload: u32,
    barrier_reader: SignalReader<'static, ()>,
    activation_writer: SignalWriter<'static, Instant>,
    activation_count: u32,
    #[cfg(feature = "profiling-on_call_producer")]
    profiler: OnCallProducerProfiler,
}

impl OnCallProducerLocals {
    #[cfg(feature = "profiling-on_call_producer")]
    pub fn new(
        barrier_reader: SignalReader<'static, ()>,
        activation_writer: SignalWriter<'static, Instant>,
        stopwatch: StopWatch<'static>,
    ) -> Self {
        Self {
            current_workload: 0,
            barrier_reader,
            activation_writer,
            activation_count: 0,
            profiler: OnCallProducerProfiler::new(stopwatch),
        }
    }

    #[cfg(not(feature = "profiling-on_call_producer"))]
    pub fn new(
        barrier_reader: SignalReader<'static, ()>,
        activation_writer: SignalWriter<'static, Instant>,
    ) -> Self {
        Self {
            current_workload: 0,
            barrier_reader,
            activation_writer,
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
        #[cfg(feature = "profiling-on_call_producer")]
        locals.profiler.reset();

        locals.barrier_reader.wait().await;

        // Signal activation to the deadline watchdog
        locals.activation_writer.write(Mono::now());
        locals.activation_count += 1;

        shared.request_buffer.lock(|buffer| {
            locals.current_workload = buffer.extract();
        });

        #[cfg(feature = "profiling-on_call_producer")]
        locals.profiler.lap(); // Lap 1: extract workload enclosing

        on_call_producer_operation(locals.current_workload);

        #[cfg(feature = "profiling-on_call_producer")]
        locals.profiler.lap(); // Lap 2: ocp_small_whetstone 

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
            }
        }
    }
}

fn on_call_producer_operation(load: u32) {
    if let Err(err) = production_workload::small_whetstone(load) {
        defmt::error!(
            "Error computing whetstone in on call producer operation: {}",
            err
        );
    }
    defmt::info!("End of sporadic activation.");
}
