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

#[cfg(feature = "profiling-external_event_server")]
pub mod external_event_server {
    use core::cmp::max;

    use rtic_monotonics::fugit::{self, ExtU64};
    use stm32f4xx_hal::dwt::StopWatch;

    use crate::profiling::Profiler;

    pub struct ExternalEventServerProfiler {
        stopwatch: StopWatch<'static>,
        wc_al_write_time: Option<fugit::MicrosDurationU64>,
        wc_ees_write_time: Option<fugit::MicrosDurationU64>,
    }

    impl ExternalEventServerProfiler {
        pub fn new(stopwatch: StopWatch<'static>) -> Self {
            Self {
                stopwatch,
                wc_al_write_time: None,
                wc_ees_write_time: None,
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
            let current_al_write_time = match (self.lap_time(2), self.lap_time(1)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_ees_write_time = self.lap_time(3);
            if let Some(current) = current_al_write_time {
                self.wc_al_write_time = Some(
                    self.wc_al_write_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_ees_write_time {
                self.wc_ees_write_time = Some(
                    self.wc_ees_write_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
        }

        fn log(&self) {
            defmt::info!(
                "
                External event server profiler:
                worst case al_write = {}
                worst case ees_write = {}
            ",
                self.wc_al_write_time,
                self.wc_ees_write_time,
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

#[cfg(feature = "profiling-regular_producer")]
pub mod regular_producer {
    use core::cmp::max;

    use rtic_monotonics::fugit::{self, ExtU64};
    use stm32f4xx_hal::dwt::StopWatch;

    use crate::profiling::Profiler;

    pub struct RegularProducerProfiler {
        stopwatch: StopWatch<'static>,
        wc_rp_smallwhetstone_time: Option<fugit::MicrosDurationU64>,
        wc_due_activation_time: Option<fugit::MicrosDurationU64>,
        wc_rb_deposit_time: Option<fugit::MicrosDurationU64>,
        wc_ocp_activation_time: Option<fugit::MicrosDurationU64>,
        wc_check_due_time: Option<fugit::MicrosDurationU64>,
        wc_alr_signal_time: Option<fugit::MicrosDurationU64>,
    }

    impl RegularProducerProfiler {
        pub fn new(stopwatch: StopWatch<'static>) -> Self {
            Self {
                stopwatch,
                wc_rp_smallwhetstone_time: None,
                wc_due_activation_time: None,
                wc_rb_deposit_time: None,
                wc_ocp_activation_time: None,
                wc_check_due_time: None,
                wc_alr_signal_time: None,
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
            let current_rp_smallwhetstone_time = self.lap_time(1);
            let current_due_activation_time = match (self.lap_time(2), self.lap_time(1)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_rb_deposit_time = match (self.lap_time(4), self.lap_time(3)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_ocp_activation_time = match (self.lap_time(5), self.lap_time(2)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_check_due_time = match (self.lap_time(6), self.lap_time(5)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            let current_alr_signal_time = match (self.lap_time(7), self.lap_time(6)) {
                (Some(end), Some(start)) => Some(end - start),
                _ => None,
            };
            if let Some(current) = current_rp_smallwhetstone_time {
                self.wc_rp_smallwhetstone_time = Some(
                    self.wc_rp_smallwhetstone_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_due_activation_time {
                self.wc_due_activation_time = Some(
                    self.wc_due_activation_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_rb_deposit_time {
                self.wc_rb_deposit_time = Some(
                    self.wc_rb_deposit_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_ocp_activation_time {
                self.wc_ocp_activation_time = Some(
                    self.wc_ocp_activation_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_check_due_time {
                self.wc_check_due_time = Some(
                    self.wc_check_due_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
            if let Some(current) = current_alr_signal_time {
                self.wc_alr_signal_time = Some(
                    self.wc_alr_signal_time
                        .map_or(current, |worst| max(current, worst)),
                );
            }
        }

        fn log(&self) {
            defmt::info!(
                "
                Regular producer profiling:
                worst case rp_small_whetstone = {}
                worst case due_activation = {}
                worst case rb_deposit = {}
                worst case ocp_activation = {}
                worst case check_due = {}
                worst case alr_signal = {}
            ",
                self.wc_rp_smallwhetstone_time,
                self.wc_due_activation_time,
                self.wc_rb_deposit_time,
                self.wc_ocp_activation_time,
                self.wc_check_due_time,
                self.wc_alr_signal_time,
            );
        }
    }
}