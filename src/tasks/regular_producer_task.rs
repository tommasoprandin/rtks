use crate::{
    app::regular_producer,
    time::Mono,
    production_workload,
    auxiliary,
};
use rtic_monotonics::{
    Monotonic,
    fugit::ExtU32,
};
use rtic::Mutex;

const PERIOD: u32 = 1000;

const REGULAR_PRODUCER_WORKLOAD: u32 = 756;
const ON_CALL_PRODUCER_WORKLOAD: u32 = 278;
const ACTIVATION_CONDITION: usize = 2;

pub async fn regular_producer_task(cx: regular_producer::Context<'_>) {
    let next_time = cx.local.next_time;
    let mut request_buffer = cx.shared.request_buffer;

    // activation_manager.activation_cyclic();

    loop {
        *next_time = Mono::now() + PERIOD.millis();

        // BEGIN REGULAR_PRODUCER_OPERATION
        if let Err(err) = production_workload::small_whetstone(REGULAR_PRODUCER_WORKLOAD) {
            defmt::error!(
                "Error computing whetstone in regular producer operation: {}",
                err
            );
        }
        if auxiliary::due_activation(ACTIVATION_CONDITION) {
            request_buffer.lock(|buffer| {
                if !buffer.deposit(ON_CALL_PRODUCER_WORKLOAD) {
                    defmt::info!("Failed sporadic activation.");
                }
            })
        }
        if auxiliary::check_due() {
            //activation_log_reader.signal;
        }
        defmt::info!("End of cyclic activation.");
        // END REGULAR_PRODUCER_OPERATION

        Mono::delay_until(*next_time).await;
    }
}