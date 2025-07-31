use core::u32;

use rtic_monotonics::{Monotonic, fugit::ExtU32 as _};
use rtic_sync::signal::{Signal, SignalReader, SignalWriter};

use crate::time::{Duration, Instant, Mono};

pub struct Deadline {
    signal: Signal<u32>,
}

impl Deadline {
    pub const fn new() -> Self {
        Self {
            signal: Signal::new(),
        }
    }

    pub fn split(
        &self,
        tag: &'static str,
        period: Duration,
    ) -> (DeadlineStopper<'_>, DeadlineMonitor<'_>) {
        let (writer, reader) = self.signal.split();
        (
            DeadlineStopper {
                signal: writer,
                activation: 0,
            },
            DeadlineMonitor {
                tag,
                period,
                misses: 0,
                signal: reader,
                next: None,
                activation: 0,
            },
        )
    }
}

pub struct DeadlineStopper<'a> {
    signal: SignalWriter<'a, u32>,
    activation: u32,
}

impl<'a> DeadlineStopper<'a> {
    pub fn done(&mut self) {
        self.activation += 1;
        self.signal.write(self.activation);
    }
}
pub struct DeadlineMonitor<'a> {
    tag: &'static str,
    period: Duration,
    misses: u32,
    signal: SignalReader<'a, u32>,
    next: Option<Instant>,
    activation: u32,
}

impl<'a> DeadlineMonitor<'a> {
    pub fn schedule(&mut self, start: Instant) {
        self.next = Some(start + self.period);
        self.activation += 1;
        let _ = self.signal.try_read();
    }

    pub fn check_and_reschedule(&mut self, now: Instant) {
        if let Some(deadline) = self.next {
            // If now >= deadline the alarm rang => we need to check
            if now >= deadline {
                match self.signal.try_read() {
                    Some(seq) => {
                        if seq != self.activation {
                            defmt::warn!("Deadline miss for {}", self.tag);
                            self.misses += 1;
                        }
                    }
                    None => {
                        defmt::warn!("Deadline miss for {}", self.tag);
                        self.misses += 1;
                    }
                }
                self.next = Some(now + self.period);
                self.activation += 1;
            }
        } else {
            defmt::warn!("Deadline {} was not scheduled", self.tag);
        }
    }
}

pub async fn deadline_watchdog(monitors: &mut [DeadlineMonitor<'_>]) -> ! {
    // Init
    if monitors.is_empty() {
        defmt::info!("No deadline monitoring");
        loop {
            Mono::delay(u32::MAX.millis()).await;
        }
    } else {
        let start = Instant::from_ticks(0);

        for monitor in monitors.iter_mut() {
            monitor.schedule(start);
        }
    }

    // Watchdog control loop
    loop {
        let earliest = monitors
            .iter()
            .filter_map(|m| m.next)
            .min()
            .expect("There should be an earliest deadline after init");
        defmt::debug!("Earliest = {}", earliest);
        Mono::delay_until(earliest).await;

        let now = Mono::now();
        for monitor in monitors.iter_mut() {
            monitor.check_and_reschedule(now);
        }
    }
}
