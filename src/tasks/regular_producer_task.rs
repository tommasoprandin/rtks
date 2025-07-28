use crate::app::regular_producer;
use crate::app::auxiliary;

const PERIOD: u32 = 1000;

const REGULAR_PRODUCER_WORKLOAD: u32 = 756;
const ON_CALL_PRODUCER_WORKLOAD: u32 = 278;
const ACTIVATION_CONDITION: usize = 2;

pub async fn regular_producer_task(cx: regular_producer::Context<'_>) {
    let mut next_time = cx.local.next_time;
    let mut request_buffer = cx.shared.request_buffer;
    // activation_manager.activation_cyclic();

    loop {
        next_time = Mono::now() + PERIOD.millis();
        regular_producer_operation();
        Mono::delay_until(next_time).await;
    }
}

fn regular_produicer_operation(request_buffer: &mut RequestBuffer) {
    // production_workload.small_whetstone(REGULAR_PRODUCER_WORKLOAD);

    if auxiliary::due_activation() {
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
}