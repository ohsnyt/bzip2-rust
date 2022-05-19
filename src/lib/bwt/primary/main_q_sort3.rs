use std::cmp::Ordering;

use log::{error, info, warn};

use super::{main_simple_sort::main_simple_sort, main_sort::QsortData};
const MAIN_QSORT_STACK_SIZE: usize = 100;
const MAIN_QSORT_SMALL_THRESH: i32 = 20;
const MAIN_QSORT_DEPTH_THRESH: i32 = 14;
//const OVERSHOOT: usize = 34;

pub(crate) fn main_q_sort3(
    qs: &mut QsortData,
) {
    while !qs.stack.is_empty() {
        if qs.stack.len() >= MAIN_QSORT_STACK_SIZE - 2 {
            error!("Excessive stack size in main_q_sort3.")
        };

        // Get the current boundaries and depth
        let (mut lo, mut hi, mut d) = qs.stack.pop().unwrap_or_default();

        // Use main_simple_sort if the context is simple (small, not deep)
        if ((hi - lo) < MAIN_QSORT_SMALL_THRESH) || (d > MAIN_QSORT_DEPTH_THRESH) {
            main_simple_sort(qs, lo, hi, d);
            // If sorting took too long, go use the fallback sorting algorithm
            if qs.budget < 0 {
                info!("Falling back to secondary sort algorithm");
                return;
            };
            continue;
        }
        // Get the approximate median value from the block data in this bucket
        // Shifting from [] to .get() did not alter speed, but did increase complexity
        let med = mmed3(
            qs.block_data[qs.bwt_ptr[lo as usize] as usize + d as usize],
            qs.block_data[qs.bwt_ptr[hi as usize] as usize + d as usize],
            qs.block_data[qs.bwt_ptr[(lo as usize + hi as usize) >> 1] as usize + d as usize],
        );

        let mut un_lo = lo;
        let mut lt_lo = lo;
        let mut un_hi = hi;
        let mut gt_hi = hi;

        loop {
            // Sort the bucket based on lt_lo and un_lo
            // This indexed versio is marginally faster than a .get() version.
            while un_hi >= un_lo {
                let n =
                    qs.block_data[qs.bwt_ptr[un_lo as usize] as usize + d as usize] as i32 - med as i32;
                if n == 0 {
                    qs.bwt_ptr.swap(un_lo as usize, lt_lo as usize);
                    lt_lo += 1;
                    un_lo += 1;
                    continue;
                };
                if n > 0 {
                    break;
                };
                un_lo += 1;
            }
            // Alternate .get() version of Sort the bucket based on lt_lo and un_lo
            // while un_hi >= un_lo {
            //     if let Some(ptr) = bwt_ptr.get(un_lo as usize) {
            //         if let Some(n) = block_data.get(*ptr as usize + d as usize) {
            //             let x = *n as i32 - med as i32;
            //             match (*n as i32 - med as i32).cmp(&0) {
            //                 Ordering::Equal => {
            //                     bwt_ptr.swap(un_lo as usize, lt_lo as usize);
            //                     lt_lo += 1;
            //                     un_lo += 1;
            //                 },
            //                 Ordering::Greater => break,
            //                 Ordering::Less => un_lo += 1,
            //             }
            //         }
            //     }
            // }
            // Sort the bucket based on gt_hi and un_hi
            while un_hi >= un_lo {
                if un_hi == 0 {
                    error!("main_q_sort3 line 64: un_hi == {}", un_hi);
                }
                let n =
                    (qs.block_data[qs.bwt_ptr[un_hi as usize] as usize + d as usize]) as i32 - med as i32;
                if n == 0 {
                    qs.bwt_ptr.swap(un_hi as usize, gt_hi as usize);
                    gt_hi -= 1;
                    un_hi -= 1;
                    continue;
                };
                if n < 0 {
                    break;
                };
                un_hi -= 1;
            }
            if un_lo > un_hi {
                break;
            };
            // Swap un_lo and un_hi, and repeat.
            qs.bwt_ptr.swap(un_lo as usize, un_hi as usize);
            un_lo += 1;
            un_hi -= 1;
        }
        if un_hi != un_lo - 1 {
            error!(
                "mainQSort3(2)b! un_hi is {}, un_lo-1 is {}",
                un_hi,
                un_lo - 1
            )
        };

        if gt_hi < lt_lo {
            qs.stack.push((lo, hi, d + 1));
            continue;
        }

        let mut n = (lt_lo as i32 - lo).min(un_lo as i32 - lt_lo as i32);
        mvswap(&mut qs.bwt_ptr, lo as i32, un_lo as i32 - n as i32, n as i32);

        let mut m = (hi - gt_hi as i32).min(gt_hi as i32 - un_hi as i32);
        mvswap(&mut qs.bwt_ptr, un_lo as i32, hi as i32 - m as i32 + 1, m as i32);

        n = lo + un_lo as i32 - lt_lo as i32 - 1;
        m = hi - (gt_hi as i32 - un_hi as i32) + 1;

        // ds: This is slightly faster than using the small vecs of next_lo, next_hi, next_d
        let mut d1 = d;
        let mut lo2 = n + 1;
        let mut hi2 = m - 1;
        let mut d2 = d + 1;

        if (n - lo) < (hi - m) {
            std::mem::swap(&mut lo, &mut m);
            std::mem::swap(&mut n, &mut hi);
            std::mem::swap(&mut d, &mut d1);
        }
        if (hi - m) < (hi2 - lo2) {
            std::mem::swap(&mut m, &mut lo2);
            std::mem::swap(&mut hi, &mut hi2);
            std::mem::swap(&mut d1, &mut d2);
        }
        if (n - lo) < (hi - m) {
            std::mem::swap(&mut lo, &mut m);
            std::mem::swap(&mut n, &mut hi);
            std::mem::swap(&mut d, &mut d1);
        }

        if (n - lo) < (hi - m) {
            error!("mainQSort3(8)a")
        };
        if (hi - m) < (hi2 - lo2) {
            error!("mainQSort3(8)b")
        };

        qs.stack.push((lo, n, d));
        qs.stack.push((m, hi, d1));
        qs.stack.push((lo2, hi2, d2));
    }
}

// ds. Cannot improve this version
/// Return the middle value of these three
fn mmed3(mut a: u16, mut b: u16, c: u16) -> u32 {
    if a > b {
        std::mem::swap(&mut a, &mut b);
    };
    if b > c {
        b = c;
        if a > b {
            b = a;
        }
    }
    b as u32
}

//  ds. This version 15% faster than previous while loop version
/// Swap n pointers starting a lo/lo_2
pub fn mvswap(bwt_ptr: &mut Vec<u32>, lo: i32, lo_2: i32, n: i32) {
    for i in 0..n {
        bwt_ptr.swap((lo + i) as usize, (lo_2 + i) as usize)
    }
}
