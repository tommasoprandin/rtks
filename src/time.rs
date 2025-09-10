use rtic_monotonics::Monotonic;
#[cfg(feature = "profiling-systick")]
use cortex_m::peripheral::DWT;

#[cfg(feature = "profiling-systick")]
static mut HCLK_MHZ: f32 = 0.0;

#[cfg(feature = "profiling-systick")]
pub fn set_hclk_mhz(hclk_mhz: f32) {
    unsafe {
        HCLK_MHZ = hclk_mhz;
    }
}

#[cfg(feature = "profiling-systick")]
fn get_hclk_mhz() -> f32 {
    unsafe { HCLK_MHZ }
}

#[cfg(feature = "profiling-systick")]
static mut DWT_REF: Option<&'static DWT> = None;

#[cfg(feature = "profiling-systick")]
pub fn set_dwt_ref(dwt_ref: &'static DWT) {
    unsafe {
        DWT_REF = Some(dwt_ref);
    }
}

#[cfg(feature = "profiling-systick")]
fn get_dwt_ref() -> &'static DWT {
    unsafe { DWT_REF.expect("DWT reference not set") }
}

#[cfg(not(feature = "profiling-systick"))]
rtic_monotonics::systick_monotonic!(Mono, 1_000);

#[cfg(feature = "profiling-systick")]
profiled_rtic_monotonics::systick_monotonic!(Mono, 1_000, get_hclk_mhz(), get_dwt_ref());

// defmt timestamp
defmt::timestamp!("{=u32:ms}", Mono::now().duration_since_epoch().to_millis());

#[cfg(not(feature = "profiling-systick"))]
pub type Instant = <Mono as rtic_monotonics::Monotonic>::Instant;

#[cfg(feature = "profiling-systick")]
pub type Instant = <Mono as profiled_rtic_monotonics::Monotonic>::Instant;