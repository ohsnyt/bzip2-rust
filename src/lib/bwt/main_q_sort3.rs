use log::{error, info};

use super::{main_simple_sort::main_simple_sort, main_sort::QSort};
const MAIN_QSORT_STACK_SIZE: usize = 100;
const MAIN_QSORT_SMALL_THRESH: i32 = 20;
const MAIN_QSORT_DEPTH_THRESH: i32 = 14;
//const OVERSHOOT: usize = 34;

pub(crate) fn main_q_sort3(
    bwt_ptr: &mut Vec<u32>,
    block_data: &[u16],
    quadrant: &mut Vec<u16>,
    budget: &mut i32,
    qs: &mut QSort,
) {
    while !qs.stack.is_empty() {
        if qs.stack.len() >= MAIN_QSORT_STACK_SIZE - 2 {
            error!("Excessive stack size in main_q_sort3.")
        };

        // Get the current boundaries and depth
        let (lo, hi, d) = qs.stack.pop().unwrap_or_default();

        // Use main_simple_sort if the context is simple (small, not deep)
        if ((hi - lo) < MAIN_QSORT_SMALL_THRESH) || (d > MAIN_QSORT_DEPTH_THRESH) {
            main_simple_sort(bwt_ptr, block_data, quadrant, qs.end, lo, hi, d, budget);
            // If sorting took too long, go use the fallback sorting algorithm
            if *budget < 0 {
                info!("Falling back to secondary sort algorithm");
                return;
            };
            continue;
        }
        //warn!("Using broken algorithm. Hi is {} and lo is {}", hi, lo);
        // Get the approximate median value from the block data in this bucket
        let med = mmed3(
            block_data[bwt_ptr[lo as usize] as usize + d as usize],
            block_data[bwt_ptr[hi as usize] as usize + d as usize],
            block_data[bwt_ptr[(lo as usize + hi as usize) >> 1] as usize + d as usize],
        );

        let mut un_lo = lo;
        let mut lt_lo = lo;
        let mut un_hi = hi;
        let mut gt_hi = hi;

        loop {
            // Sort the bucket based on lt_lo and un_lo
            while un_hi >= un_lo {
                let n =
                    block_data[bwt_ptr[un_lo as usize] as usize + d as usize] as i32 - med as i32;
                if n == 0 {
                    bwt_ptr.swap(un_lo as usize, lt_lo as usize);
                    lt_lo += 1;
                    un_lo += 1;
                    continue;
                };
                if n > 0 {
                    break;
                };
                un_lo += 1;
            }
            // Sort the bucket based on gt_hi and un_hi
            while un_hi >= un_lo {
                if un_hi == 0 {
                    error!("main_q_sort3 line 64: un_hi == {}", un_hi);
                }
                let n =
                    (block_data[bwt_ptr[un_hi as usize] as usize + d as usize]) as i32 - med as i32;
                if n == 0 {
                    bwt_ptr.swap(un_hi as usize, gt_hi as usize);
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
            bwt_ptr.swap(un_lo as usize, un_hi as usize);
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
        mvswap(bwt_ptr, lo as i32, un_lo as i32 - n as i32, n as i32);

        let mut m = (hi - gt_hi as i32).min(gt_hi as i32 - un_hi as i32);
        mvswap(bwt_ptr, un_lo as i32, hi as i32 - m as i32 + 1, m as i32);

        n = lo + un_lo as i32 - lt_lo as i32 - 1;
        m = hi - (gt_hi as i32 - un_hi as i32) + 1;

        qs.next_lo[0] = lo;
        qs.next_hi[0] = n;
        qs.next_d[0] = d;
        qs.next_lo[1] = m;
        qs.next_hi[1] = hi;
        qs.next_d[1] = d;
        qs.next_lo[2] = n + 1;
        qs.next_hi[2] = m - 1;
        qs.next_d[2] = d + 1;

        if (qs.next_hi[0] - qs.next_lo[0]) < (qs.next_hi[1] - qs.next_lo[1]) {
            qs.next_lo.swap(0, 1);
            qs.next_hi.swap(0, 1);
            qs.next_d.swap(0, 1);
        }
        if (qs.next_hi[1] - qs.next_lo[1]) < (qs.next_hi[2] - qs.next_lo[2]) {
            qs.next_lo.swap(1, 2);
            qs.next_hi.swap(1, 2);
            qs.next_d.swap(1, 2);
        }
        if (qs.next_hi[0] - qs.next_lo[0]) < (qs.next_hi[1] - qs.next_lo[1]) {
            qs.next_lo.swap(0, 1);
            qs.next_hi.swap(0, 1);
            qs.next_d.swap(0, 1);
        }

        if (qs.next_hi[0] - qs.next_lo[0]) < (qs.next_hi[1] - qs.next_lo[1]) {
            error!("mainQSort3(8)a")
        };
        if (qs.next_hi[1] - qs.next_lo[1]) < (qs.next_hi[2] - qs.next_lo[2]) {
            error!("mainQSort3(8)b")
        };

        qs.stack.push((qs.next_lo[0], qs.next_hi[0], qs.next_d[0]));
        qs.stack.push((qs.next_lo[1], qs.next_hi[1], qs.next_d[1]));
        qs.stack.push((qs.next_lo[2], qs.next_hi[2], qs.next_d[2]));
    }
}

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

/// Swap n pointers starting a lo/lo_2
fn mvswap(bwt_ptr: &mut Vec<u32>, lo: i32, lo_2: i32, n: i32) {
    let mut lo = lo as usize;
    let mut lo_2 = lo_2 as usize;
    let mut n = n;
    while n > 0 {
        bwt_ptr.swap(lo, lo_2);
        lo += 1;
        lo_2 += 1;
        n -= 1;
    }
}
