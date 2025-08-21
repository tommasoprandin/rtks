#[cfg(any(
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-regular_producer",
))]
use rtic_monotonics::fugit::{self};

#[cfg(any(
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-regular_producer",
))]
pub const WCET_THRESHOLD: u32 = 100;

#[cfg(any(
    feature = "profiling-activation_log_reader",
    feature = "profiling-external_event_server",
    feature = "profiling-on_call_producer",
    feature = "profiling-regular_producer",
))]
pub trait Profiler {
    fn reset(&mut self);
    fn lap(&mut self);
    fn lap_time(&self, lap: usize) -> Option<fugit::MicrosDurationU64>;
    fn update_wcet(&mut self);
    fn log(&self);
}

#[cfg(feature = "profiling-regular_producer")]
pub mod regular_producer {
    use core::cmp::max;

    use rtic_monotonics::fugit::{self, ExtU64};
    use stm32f4xx_hal::dwt::StopWatch;

    use crate::profiling::Profiler;

    pub struct RegularProducerProfiler {
        stopwatch: StopWatch<'static>,
        wc_workload_time: Option<fugit::MicrosDurationU64>,
        wc_activations_time: Option<fugit::MicrosDurationU64>,
    }

    impl RegularProducerProfiler {
        pub fn new(stopwatch: StopWatch<'static>) -> Self {
            Self {
                stopwatch,
                wc_workload_time: None,
                wc_activations_time: None,
            }
        }
    }

    impl Profiler for RegularProducerProfiler {
        fn reset(&mut self) {
            self.stopwatch.reset();
        }

        fn lap(&mut self) {
            self.stopwatch.lap();
        }

        fn lap_time(&self, lap: usize) -> Option<fugit::MicrosDurationU64> {
            self.stopwatch.lap_time(lap).map(|d| d.as_micros().micros())
        }

        fn update_wcet(&mut self) {
            let current_workload_time = self.lap_time(1);
            let current_activations_time = self.lap_time(2);
            if let Some(current) = current_workload_time {
                self.wc_workload_time = Some(
                    self.wc_workload_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_activations_time {
                self.wc_activations_time = Some(
                    self.wc_activations_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
        }

        fn log(&self) {
            defmt::info!(
                "
                Regular producer profiling:
                worst case workload = {}
                worst case activations = {}
            ",
                self.wc_workload_time,
                self.wc_activations_time
            );
        }
    }
}

#[cfg(feature = "profiling-external_event_server")]
pub mod external_event_server {
    use core::cmp::max;

    use rtic_monotonics::fugit::{self, ExtU64};
    use stm32f4xx_hal::dwt::StopWatch;

    use crate::profiling::Profiler;

    pub struct ExternalEventServerProfiler {
        stopwatch: StopWatch<'static>,
        wc_log_write_time: Option<fugit::MicrosDurationU64>,
    }

    impl ExternalEventServerProfiler {
        pub fn new(stopwatch: StopWatch<'static>) -> Self {
            Self {
                stopwatch,
                wc_log_write_time: None,
            }
        }
    }

    impl Profiler for ExternalEventServerProfiler {
        fn reset(&mut self) {
            self.stopwatch.reset();
        }

        fn lap(&mut self) {
            self.stopwatch.lap();
        }

        fn lap_time(&self, lap: usize) -> Option<fugit::MicrosDurationU64> {
            self.stopwatch.lap_time(lap).map(|d| d.as_micros().micros())
        }

        fn update_wcet(&mut self) {
            let current_log_write_time = self.lap_time(1);
            if let Some(current) = current_log_write_time {
                self.wc_log_write_time = Some(
                    self.wc_log_write_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
        }

        fn log(&self) {
            defmt::info!(
                "
                External event server profiler:
                worst case log write = {}
            ",
                self.wc_log_write_time,
            );
        }
    }
}

#[cfg(feature = "profiling-on_call_producer")]
pub mod on_call_producer {
    use core::cmp::max;

    use rtic_monotonics::fugit::{self, ExtU64};
    use stm32f4xx_hal::dwt::StopWatch;

    use crate::profiling::Profiler;

    pub struct OnCallProducerProfiler {
        stopwatch: StopWatch<'static>,
        wc_rb_extract_time: Option<fugit::MicrosDurationU64>,
        wc_extract_workload_time: Option<fugit::MicrosDurationU64>,
        wc_ocp_smallwhetstone_time: Option<fugit::MicrosDurationU64>,
        wc_put_line_time: Option<fugit::MicrosDurationU64>,
    }

    impl OnCallProducerProfiler {
        pub fn new(stopwatch: StopWatch<'static>) -> Self {
            Self {
                stopwatch,
                wc_rb_extract_time: None,
                wc_extract_workload_time: None,
                wc_ocp_smallwhetstone_time: None,
                wc_put_line_time: None,
            }
        }
    }

    impl Profiler for OnCallProducerProfiler {
        fn reset(&mut self) {
            self.stopwatch.reset();
        }

        fn lap(&mut self) {
            self.stopwatch.lap();
        }

        fn lap_time(&self, lap: usize) -> Option<fugit::MicrosDurationU64> {
            self.stopwatch.lap_time(lap).map(|d| d.as_micros().micros())
        }

        fn update_wcet(&mut self) {
            let current_rb_extract_time = match (self.lap_time(2), self.lap_time(1)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_extract_workload_time = match (self.lap_time(3), self.lap_time(1)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_ocp_smallwhetstone_time = match (self.lap_time(4), self.lap_time(3)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_put_line_time = match (self.lap_time(5), self.lap_time(4)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            if let Some(current) = current_rb_extract_time {
                self.wc_rb_extract_time = Some(
                    self.wc_rb_extract_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_extract_workload_time {
                self.wc_extract_workload_time = Some(
                    self.wc_extract_workload_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_ocp_smallwhetstone_time {
                self.wc_ocp_smallwhetstone_time = Some(
                    self.wc_ocp_smallwhetstone_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_put_line_time {
                self.wc_put_line_time = Some(
                    self.wc_put_line_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
        }

        fn log(&self) {
            defmt::info!(
                "
                Regular producer profiling:
                worst case rb_extract = {}
                worst case extract_workload = {}
                worst case ocp_small_whetstone = {}
                worst case put_line = {}
            ",
                self.wc_rb_extract_time,
                self.wc_extract_workload_time,
                self.wc_ocp_smallwhetstone_time,
                self.wc_put_line_time,
            );
        }
    }
}

#[cfg(feature = "profiling-activation_log_reader")]
pub mod activation_log_reader {
    use core::cmp::max;

    use rtic_monotonics::fugit::{self, ExtU64};
    use stm32f4xx_hal::dwt::StopWatch;

    use crate::profiling::Profiler;

    pub struct ActivationLogReaderProfiler {
        stopwatch: StopWatch<'static>,
        wc_alr_smallwhetstone_time: Option<fugit::MicrosDurationU64>,
        wc_al_read_time: Option<fugit::MicrosDurationU64>,
        wc_alr_read_time: Option<fugit::MicrosDurationU64>,
    }

    impl ActivationLogReaderProfiler {
        pub fn new(stopwatch: StopWatch<'static>) -> Self {
            Self {
                stopwatch,
                wc_alr_smallwhetstone_time: None,
                wc_al_read_time: None,
                wc_alr_read_time: None,
            }
        }
    }

    impl Profiler for ActivationLogReaderProfiler {
        fn reset(&mut self) {
            self.stopwatch.reset();
        }

        fn lap(&mut self) {
            self.stopwatch.lap();
        }

        fn lap_time(&self, lap: usize) -> Option<fugit::MicrosDurationU64> {
            self.stopwatch.lap_time(lap).map(|d| d.as_micros().micros())
        }

        fn update_wcet(&mut self) {
            let current_alr_smallwhetstone_time = self.lap_time(1);
            let current_al_read_time = match (self.lap_time(3), self.lap_time(2)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_alr_read_time = match(self.lap_time(4), self.lap_time(1)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            if let Some(current) = current_alr_smallwhetstone_time {
                self.wc_alr_smallwhetstone_time = Some(
                    self.wc_alr_smallwhetstone_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_al_read_time {
                self.wc_al_read_time = Some(
                    self.wc_al_read_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_alr_read_time {
                self.wc_alr_read_time = Some(
                    self.wc_alr_read_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
        }

        fn log(&self) {
            defmt::info!(
                "
                Activation Log Reader profiling:
                worst case alr_smallwhetstone = {}
                worst case al_read = {}
                worst case alr_read = {}
            ",
                self.wc_alr_smallwhetstone_time,
                self.wc_al_read_time,
                self.wc_alr_read_time
            );
        }
    }
}