use rtic_monotonics::fugit::{self, ExtU64};
use stm32f4xx_hal::dwt::StopWatch;

pub trait Profiler {
    fn reset(&mut self);
    fn lap(&mut self);
    fn lap_time(&self, lap: usize) -> Option<fugit::MicrosDurationU64>;
    fn log(&self);
}

#[cfg(any(
    feature = "profiling-regular_producer",
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-activation_log",
    feature = "profiling-request_buffer",
))]
pub struct StopwatchProfiler {
    name: &'static str,
    stopwatch: StopWatch<'static>,
}

#[cfg(any(
    feature = "profiling-regular_producer",
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-activation_log",
    feature = "profiling-request_buffer",
))]
impl StopwatchProfiler {
    pub fn new(name: &'static str, stopwatch: StopWatch<'static>) -> Self {
        Self { name, stopwatch }
    }
}

#[cfg(any(
    feature = "profiling-regular_producer",
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-activation_log",
    feature = "profiling-request_buffer",
))]
impl Profiler for StopwatchProfiler {
    fn reset(&mut self) {
        self.stopwatch.reset();
    }

    fn lap(&mut self) {
        self.stopwatch.lap();
    }

    fn lap_time(&self, lap: usize) -> Option<fugit::MicrosDurationU64> {
        self.stopwatch
            .lap_time(lap)
            .map(|time| time.as_micros().micros())
    }

    fn log(&self) {
        let mut lap = 1;
        while let Some(time) = self.stopwatch.lap_time(lap) {
            defmt::info!("{} lap {} = {}", self.name, lap, time.as_micros());
            lap += 1;
        }
    }
}

#[cfg(not(all(
    feature = "profiling-regular_producer",
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-activation_log",
    feature = "profiling-request_buffer",
)))]
pub struct NoOpProfiler;

#[cfg(not(all(
    feature = "profiling-regular_producer",
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-activation_log",
    feature = "profiling-request_buffer",
)))]
impl NoOpProfiler {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(all(
    feature = "profiling-regular_producer",
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-activation_log",
    feature = "profiling-request_buffer",
)))]
impl Profiler for NoOpProfiler {
    fn reset(&mut self) {}

    fn lap(&mut self) {}

    fn lap_time(&self, _lap: usize) -> Option<fugit::MicrosDurationU64> {
        None
    }

    fn log(&self) {}
}

#[cfg(feature = "profiling-regular_producer")]
pub type RegularProducerProfiler = StopwatchProfiler;
#[cfg(not(feature = "profiling-regular_producer"))]
pub type RegularProducerProfiler = NoOpProfiler;

#[cfg(feature = "profiling-activation_log_reader")]
pub type ActivationLogReaderProfiler = StopwatchProfiler;
#[cfg(not(feature = "profiling-activation_log_reader"))]
pub type ActivationLogReaderProfiler = NoOpProfiler;

#[cfg(feature = "profiling-external_event_server")]
pub type ExternalEventServerProfiler = StopwatchProfiler;
#[cfg(not(feature = "profiling-external_event_server"))]
pub type ExternalEventServerProfiler = NoOpProfiler;

#[cfg(feature = "profiling-on_call_producer")]
pub type OnCallProducerProfiler = StopwatchProfiler;
#[cfg(not(feature = "profiling-on_call_producer"))]
pub type OnCallProducerProfiler = NoOpProfiler;

#[cfg(feature = "profiling-activation_log")]
pub type ActivationLogProfiler = StopwatchProfiler;
#[cfg(not(feature = "profiling-activation_log"))]
pub type ActivationLogProfiler = NoOpProfiler;

#[cfg(feature = "profiling-request_buffer")]
pub type RequestBufferProfiler = StopwatchProfiler;
#[cfg(not(feature = "profiling-request_buffer"))]
pub type RequestBufferProfiler = NoOpProfiler;
