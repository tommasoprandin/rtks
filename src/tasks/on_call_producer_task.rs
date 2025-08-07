use crate::{
    production_workload, 
    deadline::DeadlineObject,
    time::{Mono, Instant}};
use rtic_sync::signal::{SignalReader, SignalWriter};
use rtic::Mutex;  
use rtic_monotonics::Monotonic;


pub const DEADLINE: u32 = 800;

pub async fn on_call_producer_task(
    request_buffer: &mut impl Mutex<T = crate::resources::request_buffer::RequestBuffer>,
    current_workload: &mut u32,
    barrier_reader: &mut SignalReader<'static, ()>,
    deadline_writer: &mut SignalWriter<'static, Instant>,
    deadline: &mut impl rtic::Mutex<T = DeadlineObject>,
    activation_count: &mut u32
) -> ! {
    loop {
        barrier_reader.wait().await;
        // start deadline
        deadline_writer.write(Mono::now());
        *activation_count += 1;

        request_buffer.lock( |buffer| {
            *current_workload = buffer.extract();
        });
        on_call_producer_operation(*current_workload);

        // Cancel deadline
        deadline.lock( |deadline| {
            deadline.cancel_deadline(*activation_count);
        });
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