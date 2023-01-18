use log::error;

/// Three-way quick sort used by fallback_sort
pub fn fallback_q_sort3(freq_map: &mut [u32], block_data: &[u16], lo_st: i32, hi_st: i32) {
    const FALLBACK_QSORT_SMALL_THRESH: usize = 10;
    const FALLBACK_QSORT_STACK_SIZE: usize = 100;
    let mut stack: Vec<(i32, i32)> = Vec::with_capacity(10);

    // C version uses this stack to push and pop ranges to sort. It MAY be
    // possible to just push and pop tuples to a vec, but this needs to be examined.
    stack.push((lo_st, hi_st));

    let mut ratio = 0;
    while !stack.is_empty() {
        // Error on excessive stack size
        if stack.len() >= FALLBACK_QSORT_STACK_SIZE - 1 {
            error!(
                "Fallback_q_sort stack now {} over intended max size of {}. Error 1004",
                stack.len() - FALLBACK_QSORT_STACK_SIZE,
                FALLBACK_QSORT_STACK_SIZE
            );
            // ds: I want to see the error - and since I moved to a vec, this isn't a problem
            // unless it grows wildly.
            if stack.len() >= FALLBACK_QSORT_STACK_SIZE + 20 {
                std::process::exit(1004)
            };
        }

        let (lo, hi) = stack.pop().unwrap();

        // use quicksort if the slice between hi and lo is less than 10 (FALLBACK_QSORT_STACK_SIZE)
        // NOTE: Julian called fallback_simple_sort() here. I use .sort_by() instead
        if hi - lo < 10 + FALLBACK_QSORT_SMALL_THRESH as i32 {
            // ...but only if it is a valid, non-empty slice
            if hi > 0 && lo >= 0 && hi - lo > 0 {
                freq_map[(lo as usize)..=(hi as usize)]
                    .sort_unstable_by(|a, b| block_data[*a as usize].cmp(&block_data[*b as usize]));
            }

            // DEBUG
            if freq_map[6553] == 0 {
                println!("Pause here")
            }
            continue;
        }
        // Use the following to sort larger slices
        /* Julian's notes:
        Random partitioning.  Median of 3 sometimes fails to avoid bad cases.
        Median of 9 seems to help but looks rather expensive.  This too seems to
        work but is cheaper.  Guidance for the magic constants 7621 and 32768 is
        taken from Sedgewick's algorithms book, chapter 35.
        */
        let mut n;
        // ds changed "m" to "median" below, but that may not be the best variable name.
        ratio = ((ratio * 7621) + 1) % 32768;
        let median = match ratio % 3 {
            0 => block_data[freq_map[lo as usize] as usize],
            1 => block_data[freq_map[((lo + hi) as usize) >> 1] as usize],
            _ => block_data[freq_map[hi as usize] as usize],
        };

        let mut un_lo = lo;
        let mut lt_lo = lo;
        let mut un_hi = hi;
        let mut gt_hi = hi;

        loop {
            loop {
                if un_lo > un_hi {
                    break;
                };
                n = (block_data[freq_map[un_lo as usize] as usize] as i32) - median as i32;
                if n == 0 {
                    freq_map.swap(un_lo as usize, lt_lo as usize);
                    lt_lo += 1;
                    un_lo += 1;
                    continue;
                };
                if n > 0 {
                    break;
                }
                un_lo += 1;
            }
            loop {
                if un_lo > un_hi {
                    break;
                }
                n = (block_data[freq_map[un_hi as usize] as usize] as i32) - median as i32;
                if n == 0 {
                    freq_map.swap(un_hi as usize, gt_hi as usize);
                    //info!("b Swapped freq_map indecies {} and {}", un_hi, gt_hi);
                    gt_hi -= 1;
                    un_hi -= 1;
                    continue;
                };
                if n < 0 {
                    break;
                }
                un_hi -= 1;
            }

            if un_lo > un_hi {
                break;
            };

            freq_map.swap(un_lo as usize, un_hi as usize);
            //info!("c Swapped freq_map indecies {} and {}", un_lo, un_hi);
            un_lo += 1;
            un_hi -= 1;
        }

        if un_hi != un_lo - 1 {
            error!("fallbackQSort3(2)") // whatever this is. ds
        }

        if gt_hi < lt_lo {
            continue;
        }

        let n = (lt_lo - lo).min(un_lo - lt_lo);
        for i in 0..n {
            freq_map.swap((i + lo) as usize, (i + un_lo - n) as usize);
        }

        let m = (hi - gt_hi).min(gt_hi - un_hi);
        for i in 0..m {
            freq_map.swap((i + un_lo) as usize, (i + hi - m + 1) as usize);
        }

        let n = lo + un_lo - lt_lo - 1;
        let m = hi - (gt_hi - un_hi) + 1;

        if n - lo > hi - m {
            stack.push((lo, n));
            stack.push((m, hi));
        } else {
            stack.push((m, hi));
            stack.push((lo, n));
        }
    }
}
