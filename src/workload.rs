use core::{error::Error, fmt::Display};

use libm::{cosf, expf, fabsf, logf, sinf, sqrtf};

// Type aliases for easy configuration
type WhetFloat = f32;
type WhetInt = i32;

// Constants from the original Algol Whetstone program
const T: WhetFloat = 0.499975;
const T1: WhetFloat = 0.50025;
const T2: WhetFloat = 2.0;
const N8: usize = 10; // Loop iteration count for module 8
const N9: usize = 7; // Loop iteration count for module 9
const VALUE: WhetFloat = 0.941377; // Value calculated in main loop
const TOLERANCE: WhetFloat = 0.00001; // Determined by interval arithmetic

// Custom error type for workload failure
#[derive(Debug, defmt::Format)]
pub struct WorkloadFailure {
    actual: WhetFloat,
    expected: WhetFloat,
}

impl Display for WorkloadFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Whetstone computation failed, actual = {}, expected = {}",
            self.actual, self.expected
        )
    }
}

impl Error for WorkloadFailure {}

pub fn small_whetstone(kilo_whets: u32) -> Result<(), WorkloadFailure> {
    let mut ij: WhetInt = 1;
    let mut ik: WhetInt = 2;
    let mut il: WhetInt = 3;

    let y: WhetFloat = 1.0; // Constant within loop
    let mut z: WhetFloat;
    let mut sum: WhetFloat = 0.0; // Accumulates value of Z
    let mut e1: [WhetFloat; N9] = [0.0; N9]; // Array indexed 0..N9-1 (equivalent to 1..N9 in Ada)

    // Helper function to clear array
    fn clear_array(arr: &mut [WhetFloat; N9]) {
        for elem in arr.iter_mut() {
            *elem = 0.0;
        }
    }

    // P0 procedure - handles array bounds checking like Ada version
    fn p0(ij: WhetInt, ik: WhetInt, il: WhetInt, i: usize, e1: &mut [WhetFloat; N9]) {
        // Convert to 0-based indexing and check bounds
        let ij_idx = (ij - 1) as usize;
        let ik_idx = (ik - 1) as usize;
        let il_idx = (il - 1) as usize;

        if ij_idx < N9 && ik_idx < N9 && il_idx < N9 && i < N9 {
            e1[ij_idx] = e1[ik_idx];
            e1[ik_idx] = e1[il_idx];
            e1[i] = e1[ij_idx];
        }
    }

    // P3 procedure
    fn p3(x: WhetFloat, y: WhetFloat, z: &mut WhetFloat) {
        let xtemp: WhetFloat = T * (*z + x);
        let ytemp: WhetFloat = T * (xtemp + y);
        *z = (xtemp + ytemp) / T2;
    }

    // Main benchmark loop
    for _outer_loop_var in 1..=kilo_whets {
        clear_array(&mut e1);

        // Module 6: Integer arithmetic
        defmt::trace!(
            "Iteration: {}, ij = {}, ik = {}, il = {}",
            _outer_loop_var,
            ij,
            ik,
            il
        );
        ij = (ik - ij) * (il - ik);
        ik = il - (ik - ij);
        il = (il - ik) * (ik + il);
        defmt::trace!(
            "Iteration: {}, ij = {}, ik = {}, il = {}",
            _outer_loop_var,
            ij,
            ik,
            il
        );

        // Convert to 0-based indexing and handle bounds
        let il_idx = (il - 2) as usize;
        e1[il_idx] = (ij + ik + il) as WhetFloat;

        let ik_idx = (ik - 2) as usize;
        if ik_idx < N9 {
            e1[ik_idx] = sinf((il as WhetFloat).into()) as WhetFloat;
        } else {
            e1[N9 - 1] = sinf((il as WhetFloat).into()) as WhetFloat;
        }

        // Module 8: Procedure calls
        z = e1[3]; // E1(4) in Ada is E1[3] in 0-based indexing
        for inner_loop_var in 1..=N8 {
            p3(y * inner_loop_var as WhetFloat, y + z, &mut z);
        }

        // Second version of Module 6
        ij = il - (il - 3) * ik;
        il = (il - ik) * (ik - ij);
        ik = (il - ik) * ik;
        defmt::trace!(
            "Iteration: {}, ij = {}, ik = {}, il = {}",
            _outer_loop_var,
            ij,
            ik,
            il
        );

        let il_idx = (il - 2) as usize;
        if il_idx < N9 {
            e1[il_idx] = (ij + ik + il) as WhetFloat;
        } else {
            e1[N9 - 1] = (ij + ik + il) as WhetFloat;
        }

        let ik_idx = ik as usize; // ik + 1 converted to 0-based
        if ik_idx < N9 {
            e1[ik_idx] = fabsf(cosf(z.into())) as WhetFloat;
        } else {
            e1[N9 - 1] = fabsf(cosf(z.into())) as WhetFloat;
        }

        // Module 9: Array references
        // Using while loop like the Ada version to allow I to be used in P0
        for i in 0..N9 {
            // equivalent to I <= N9 in Ada
            p0(ij, ik, il, i, &mut e1);
        }

        // Module 11: Standard mathematical functions
        if e1[N9 - 1] > 0.0 {
            // E1(N9) in Ada is E1[N9-1] in 0-based indexing
            z = sqrtf(expf(logf(e1[N9 - 1].into()) / T1)) as WhetFloat;
        } else {
            z = sqrtf(expf(logf(1.1) / T1)) as WhetFloat;
        }

        sum += z;

        // Check the current value of the loop computation
        defmt::debug!("|z - value| = {}", (z - VALUE).abs());
        if (z - VALUE).abs() > TOLERANCE {
            sum = 2.0 * sum; // Forces error at end
            ij += 1; // Prevents optimization
        }
    }

    // Self-validation check
    let actual = sum / kilo_whets as WhetFloat - VALUE;
    let expected = kilo_whets as WhetFloat;
    if actual.abs() > TOLERANCE * expected {
        return Err(WorkloadFailure { actual, expected });
    }

    Ok(())
}
