use crate::{
    production_workload, 
    activation_manager,
    deadline::DeadlineProtectedObject,
    time::{Mono, Instant}};
use rtic_sync::signal::{SignalReader, SignalWriter};
use rtic::Mutex;  
use rtic_monotonics::Monotonic;
#[cfg(feature = "profiling-on_call_producer")]
use stm32f4xx_hal::dwt::{Dwt, StopWatch};
#[cfg(feature = "profiling-on_call_producer")]
use crate::WCET_THRESHOLD;

pub const DEADLINE: u32 = 800;

pub async fn on_call_producer_task(
    request_buffer: &mut impl Mutex<T = crate::resources::request_buffer::RequestBuffer>,
    current_workload: &mut u32,
    barrier_reader: &mut SignalReader<'static, ()>,
    activation_writer: &mut SignalWriter<'static, Instant>,
    deadline_protected_object: &mut impl rtic::Mutex<T = DeadlineProtectedObject>,
    activation_count: &mut u32,
    #[cfg(feature = "profiling-on_call_producer")] wc_extract_workload: &mut u32,
    #[cfg(feature = "profiling-on_call_producer")] wc_ocp_small_whetstone: &mut u32,
    #[cfg(feature = "profiling-on_call_producer")] times: &mut [u32; 2],
    #[cfg(feature = "profiling-on_call_producer")] stopwatch: &mut StopWatch<'static>,
) -> ! {
    activation_manager::activation_sporadic().await;
    loop {
        #[cfg(feature = "profiling-on_call_producer")]
        stopwatch.reset();

        barrier_reader.wait().await;

        // Signal activation to the deadline watchdog
        activation_writer.write(Mono::now());
        *activation_count += 1;

        request_buffer.lock( |buffer| {
            *current_workload = buffer.extract();
        });
        #[cfg(feature = "profiling-on_call_producer")]
        stopwatch.lap(); // Lap 1: extract workload enclosing
        on_call_producer_operation(*current_workload);
        #[cfg(feature = "profiling-on_call_producer")]
        stopwatch.lap(); // Lap 2: ocp_small_whetstone 

        // Cancel deadline
        deadline_protected_object.lock( |dpo| {
            dpo.cancel_deadline(*activation_count);
        });

        #[cfg(feature = "profiling-on_call_producer")]
        {
            wc_extract_workload = wc_extract_workload.max(stopwatch.lap_time(1).unwrap().as_micros());
            wc_ocp_small_whetstone = wc_ocp_small_whetstone.max(stopwatch.lap_time(2).unwrap().as_micros() - stopwatch.lap_time(1).unwrap().as_micros());
            if activation_count == WCET_THRESHOLD {
                defmt::info!("OCP Profiling:\t
                extract workload = {}us\t
                ocp small whetstone = {}us", 
                wc_extract_workload, 
                wc_ocp_small_whetstone);
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