use log::{error, warn};

use super::main_sort::QsortData;

/// Revised C version - Rust iterated versions were slower
pub fn main_gtu(
    i1: i32,
    i2: i32,
    qs: &mut QsortData,
) -> bool {
    if i1 == i2 {
        error!("mainGtU error")
    }
    let mut a = i1 as usize;
    let mut b = i2 as usize;

    macro_rules! check_bd {
        () => {
            if let Some(result) = check_data(&qs.block_data, a, b) {
                return result;
            }
            a += 1;
            b += 1;
        };
    }
    macro_rules! check_bdq {
        () => {
            if let Some(result) = check_data(&qs.block_data, a, b) {
                return result;
            }
            if let Some(result) = check_data(&qs.quadrant, a, b) {
                return result;
            }
            a += 1;
            b += 1;
        };
    }

    // Check block data 12 times
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();
    check_bd!();

    let mut k: i32 = qs.end as i32 + 8;
    while k >= 0 {
        // Check block data then quadrant data 8 times
        check_bdq!();
        check_bdq!();
        check_bdq!();
        check_bdq!();
        check_bdq!();
        check_bdq!();
        check_bdq!();
        check_bdq!();

        // Wrap around the end of the block.
        // (Note: the block_data and quadrant are extened past end.)
        if a > qs.end {
            a -= qs.end
        }
        if b > qs.end {
            b -= qs.end
        }
        k -= 8;
        qs.budget -= 1;
    }
    false
}

#[inline(always)]
fn check_data(data: &[u16], a: usize, b: usize) -> Option<bool> {
    if data[a] != data[b] {
        Some(data[a] > data[b])
    } else {
        None
    }
}
