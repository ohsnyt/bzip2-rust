use log::{error};

use super::main_sort::QsortData;

/// Revised C version - Rust iterated versions were slower
pub fn main_gtu(mut a: usize, mut b: usize, qs: &mut QsortData) -> bool {
    if a == b {
        error!("mainGtU error")
    }

    // I thought maybe doing this by slice could be faster. No. It is slower.
    macro_rules! check_bd {
        () => {
            if &qs.block_data[a] != &qs.block_data[b] {
                return &qs.block_data[a] > &qs.block_data[b];
            } else {
                a += 1;
                b += 1;
            }
        };
    }
    macro_rules! check_bdq {
        () => {
            if &qs.block_data[a] != &qs.block_data[b] {
                return &qs.block_data[a] > &qs.block_data[b];
            } else if &qs.quadrant[a] != &qs.quadrant[b] {
                return &qs.quadrant[a] > &qs.quadrant[b];
            } else {
                a += 1;
                b += 1;
            }
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
        // (Note: the block_data and quadrant are extended past end.)
        a %= qs.end;
        b %= qs.end;
        k -= 8;
        qs.budget -= 1;
    }
    false
}
