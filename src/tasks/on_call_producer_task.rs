use crate::production_workload;
use rtic_sync::signal::SignalReader;
use rtic::Mutex;

pub async fn on_call_producer_task(
    request_buffer: &mut impl Mutex<T = crate::resources::request_buffer::RequestBuffer>,
    current_workload: &mut u32,
    barrier_reader: &mut SignalReader<'static, ()>
) -> ! {
    loop {
        defmt::info!("Waiting for sporadic activation through signal...");
        barrier_reader.wait().await;
        defmt::info!("Start of sporadic activation");
        request_buffer.lock( |buffer| {
            *current_workload = buffer.extract();
        });
        on_call_producer_operation(*current_workload);
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