use rtic_monotonics::Monotonic;

// Timer interrupt setup and timer type creation
// stm32_tim2_monotonic!(Mono, 1_000);
rtic_monotonics::systick_monotonic!(Mono, 1_000);
// defmt timestamp
defmt::timestamp!("{=u32:ms}", Mono::now().duration_since_epoch().to_millis());

pub type Instant = <Mono as rtic_monotonics::Monotonic>::Instant;
pub type Duration = <Mono as rtic_monotonics::Monotonic>::Duration;