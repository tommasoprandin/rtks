use crate::app::on_call_producer;
use rtic::Mutex;

pub fn on_call_producer_task(cx: on_call_producer::Context) {    
    let mut request_buffer = cx.shared.request_buffer;
    let current_workload = cx.local.current_workload;

    // activation_manager.activation_sporadic();

    loop {
        request_buffer.lock( |buffer| {
            *current_workload = buffer.extract();
        });
        on_call_producer_operation(*current_workload);
    }
} 

fn on_call_producer_operation(load: u32) {
    defmt::info!("On_Call_Producer is processing workload: {}", load);
    // production_workload.small_whetstone(load);
    defmt::info!("End of sporadic activation.");
}