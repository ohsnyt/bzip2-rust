use log::{error, trace};

use super::main_simple_sort::main_simple_sort;

pub(crate) fn main_q_sort3(
    bwt_ptr: &mut Vec<u32>,
    block_data: &[u16],
    quadrant: &mut Vec<u16>,
    end: usize,
    mut lo: i32,
    mut hi: i32,
    mut d: i32,
    budget: &mut i32,
) {
    const MAIN_QSORT_STACK_SIZE: usize = 100;
    const MAIN_QSORT_SMALL_THRESH: i32 = 20;
    const MAIN_QSORT_DEPTH_THRESH: i32 = 14;

    let mut stack_lo = [0_i32; MAIN_QSORT_STACK_SIZE];
    let mut stack_hi = [0_i32; MAIN_QSORT_STACK_SIZE];
    let mut stack_d = [0_i32; MAIN_QSORT_STACK_SIZE];

    let mut next_lo = [0_i32; 3];
    let mut next_hi = [0_i32; 3];
    let mut next_d = [0_i32; 3];

    let mut sp = 0;
    macro_rules! mpush {
        ($lz:expr,$hz:expr,$dz:expr) => {
            stack_lo[sp as usize] = $lz;
            stack_hi[sp as usize] = $hz;
            stack_d[sp as usize] = $dz;
            sp += 1;
        };
    }

    macro_rules! mpop {
        ($lz:expr,$hz:expr,$dz:expr) => {
            sp -= 1;
            $lz = stack_lo[sp as usize];
            $hz = stack_hi[sp as usize];
            $dz = stack_d[sp as usize];
        };
    }

    mpush!(lo, hi, d);

    while sp > 0 {
        if sp >= MAIN_QSORT_STACK_SIZE - 2 {
            error!("Something wrong in main_q_sort3.")
        };

        mpop!(lo, hi, d);
        if (hi - lo < MAIN_QSORT_SMALL_THRESH) || (d > MAIN_QSORT_DEPTH_THRESH) {
            trace!("Going to main_simple_sort");
            main_simple_sort(bwt_ptr, block_data, quadrant, end, lo, hi, d, budget);
            if *budget < 0 {
                return;
            };
            continue;
        }

        let med = mmed3(
            block_data[bwt_ptr[lo as usize] as usize + d as usize],
            block_data[bwt_ptr[hi as usize] as usize + d as usize],
            block_data[bwt_ptr[(lo as usize + hi as usize) >> 1] as usize + d as usize],
        );

        let mut un_lo = lo as i32;
        let mut lt_lo = un_lo;
        let mut un_hi = hi as i32;
        let mut gt_hi = un_hi;

        loop {
            loop {
                if un_lo > un_hi {
                    break;
                };
                let n = block_data[bwt_ptr[un_lo as usize] as usize + d as usize] as i32
                    - med as i32;
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
            loop {
                if un_lo > un_hi {
                    break;
                };
                if un_hi >= bwt_ptr.len() as i32 {
                    println!("Over-read in q_sort3. line 97. un_hi was {} and bwt_ptr is only {} long.", un_hi, bwt_ptr.len());
                }
                if bwt_ptr[un_hi as usize] + d as u32 >= block_data.len() as u32 {
                    println!("Over-read in q_sort3. line 100");
                }
                let n = (block_data[bwt_ptr[un_hi as usize] as usize + d as usize])
                    as i32
                    - med as i32;
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
            bwt_ptr.swap(un_lo as usize, un_hi as usize);
            un_lo += 1;
            un_hi -= 1;
        }
        if un_hi != un_lo - 1 {
            error!("mainQSort3(2)")
        };

        if gt_hi < lt_lo {
            mpush!(lo, hi, d + 1);
            continue;
        }

        let mut n = (lt_lo as i32 - lo).min(un_lo as i32 - lt_lo as i32);
        mvswap(bwt_ptr, lo as i32, un_lo as i32 - n as i32, n as i32);

        let mut m = (hi - gt_hi as i32).min(gt_hi as i32 - un_hi as i32);
        mvswap(bwt_ptr, un_lo as i32, hi as i32 - m as i32 + 1, m as i32);

        n = lo + un_lo as i32 - lt_lo as i32 - 1;
        m = hi - (gt_hi as i32 - un_hi as i32) + 1;

        next_lo[0] = lo;
        next_hi[0] = n;
        next_d[0] = d;
        next_lo[1] = m;
        next_hi[1] = hi;
        next_d[1] = d;
        next_lo[2] = n + 1;
        next_hi[2] = m - 1;
        next_d[2] = d + 1;

        if (next_hi[0] - next_lo[0]) < (next_hi[1] - next_lo[1]) {
            next_lo.swap(0, 1);
            next_hi.swap(0, 1);
            next_d.swap(0, 1);
        }
        if (next_hi[1] - next_lo[1]) < (next_hi[2] - next_lo[2]) {
            next_lo.swap(1, 2);
            next_hi.swap(1, 2);
            next_d.swap(1, 2);
        }
        if (next_hi[0] - next_lo[0]) < (next_hi[1] - next_lo[1]) {
            next_lo.swap(0, 1);
            next_hi.swap(0, 1);
            next_d.swap(0, 1);
        }

        if (next_hi[0] - next_lo[0]) < (next_hi[1] - next_lo[1]) {
            error!("mainQSort3(8)")
        };
        if (next_hi[1] - next_lo[1]) < (next_hi[2] - next_lo[2]) {
            error!("mainQSort3(8)")
        };
        mpush!(next_lo[0], next_hi[0], next_d[0]);
        mpush!(next_lo[1], next_hi[1], next_d[1]);
        mpush!(next_lo[2], next_hi[2], next_d[2]);
    }
}

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

fn mvswap(bwt_ptr: &mut Vec<u32>, lo: i32, lo_2: i32, n: i32) {
    let mut l1 = lo as usize;
    let mut l2 = lo_2 as usize;
    let mut n1 = n;
    while n1 > 0 {
        bwt_ptr.swap(l1, l2);
        l1 += 1;
        l2 += 1;
        n1 -= 1;
    }
}
