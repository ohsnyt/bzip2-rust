use log::{error, info};

use crate::{compression::compress::Block, tools::freq_count::freqs};

/// Sort function for blocks if the block size is less than 10k size or the block contains highly repetitive data.
/// Rewritten deeply by ds.
//pub fn fallback_sort_ds(mut data: &mut [u8]) -> (u32, Vec<u8>) {
pub fn fallback_sort_ds(block: &mut Block) {
    // I refer to the end of the data a lot, so for readability...
    let end = block.data.len();
    // Julian shifted to 257 symbols. I'm trying to stay within 256.

    log::warn!("using fallback_sort_ds");
    info!("     bucket sorting ...");
    // Create sentinel bits to mark the edges of all "buckets" for sorting.
    // Each bit in the sentinels vec refers to one u8 in the input data
    // Add one u32 to the vec for rounding up.
    // CHECK Add two u32s for block-end detection. (Not sure why Rust would need this.)
    let mut sentinels: Vec<u32> = vec![0_u32; 3 + (end / 32)];

    // Macros set/clear sentinal bits in the sentinels used in sorting.
    // There are 32 sentinal bits per bucket (32 = 2^5). Therefore shifting the index number right
    // by 5 bits will give us the bucket number, and the rightmost 5 bits will point to the
    // position within the bucket (1 << ($zz & 31)).
    macro_rules! set_sentinel {
        ($zz:expr) => {
            sentinels[$zz as usize >> 5] |= (1 << ($zz & 31))
        };
    }
    // macro_rules! clear_sentinel {
    //     ($zz:expr) => {
    //         sentinels[$zz as usize >> 5] &= !(1 << ($zz & 31))
    //     };
    // }
    macro_rules! is_set_sentinel {
        ($zz:expr) => {
            (sentinels[$zz as usize >> 5] & (1 << ($zz & 31)) as u32) > 0
        };
    }
    macro_rules! is_not_set_sentinel {
        ($zz:expr) => {
            (sentinels[$zz as usize >> 5] & (1 << ($zz & 31)) as u32) <= 0
        };
    }
    // Build a frequency table (we need it at the end to rebuild data) and turn it into a cumulative sum table.
    let mut freqs = freqs(&block.data);
    let mut sum_freq = freqs.clone();
    sum_freq.iter_mut().fold(0, |acc, x| {
        *x += acc;
        *x
    });

    /*
    Now build an index (map of each u8 if sorted lexically) to each u8 in the input such that index[0]
    points to the first instance of the lexically lowest u8 in the input and index[index.len()] points
    to the last instance of the largest u8 in the input.
    */
    let mut u8_index =
        block
            .data
            .iter()
            .enumerate()
            .fold(vec![0_usize; end], |mut map, (idx, byte)| {
                map[(sum_freq[*byte as usize] - 1) as usize] = idx;
                sum_freq[*byte as usize] -= 1;
                map
            });

    // Set a sentinel bit for each change in sum_freq. Sequences of set bits indicate unsorted slices.
    sum_freq.iter().for_each(|&el| set_sentinel!(el as usize));

    // Set mark/space sentinel bits at the end of the freq bits for block-end detection
    // (This sets a sequence of marks and space bits (101010...) after the regular sentinal bits.)
    (0..32).for_each(|i| {
        set_sentinel!(end + 2 * i);
    });

    /*
    Julian's note: Inductively refine the sentinels.  Kind-of an "exponential radix
    sort" (!), inspired by the  Manber-Myers suffix array construction algorithm.

    We will call the sort function log(N) times in this loop below. N will be determined by whether
    we still have "sentinels" that need to be sorted.

    We increase the number of characters to be considered each time by the power of 2.
    Set depth for the first iteration to 1
    */
    let mut depth = 1;

    loop {
        // Begin to count how many elements are not sorted
        // Initialize a counter to count how many unsorted elements exist at this level - used in reporting
        let mut unsorted_found = 0;
        // Initialize left and right boundary indices
        let mut left = 0;
        let mut right;
        loop {
            // Set left/right boundary indices. If not left, break the loop
            match (left..end).position(|i| is_not_set_sentinel!(i)) {
                None => break,
                Some(n) => left += n - 1,
            };
            match ((left + 1)..end).position(|i| is_set_sentinel!(i)) {
                None => right = end - 1,
                Some(n) => right = left + n,
            };
            // Indicate how much work has been done.
            unsorted_found += right - left + 1;
            // Sort if more than one element in the current slice
            if right - left > 1 {
                fallback_q_sort3(&mut u8_index, &mut block.data, left, right, depth)
            }

            // Rescan to set new sentinal bits within the slice we just sorted.
            let mut first = block.data[u8_index[left]];
            set_sentinel!(left);
            for i in left..=right {
                let next = block.data[u8_index[i]];
                if first != next {
                    set_sentinel!(i);
                    first = block.data[u8_index[i]]
                }
            }
            left = right + 1;
        }
        info!(
            "depth {:>7} had {} unresolved strings",
            depth, unsorted_found
        );

        depth <<= 1;
        if depth > end || unsorted_found == 0 {
            break;
        };
    }

    regenerate_bwt_data(&mut block.data, &mut freqs, &u8_index);

    // Generate key and BWT output
    let mut bwt = vec![0_u8; end];
    u8_index.iter().enumerate().for_each(|(i, &el)| {
        if el == 0 {
            block.key = i as u32;
            bwt[i] = block.data[end - 1]
        } else {
            bwt[i] = block.data[el - 1]
        };
    });
    // Move bwt to block.data
    block.data.clear();
    block.data = bwt;
}

/// Rebuild the original data - IS THIS NEEDED?.
fn regenerate_bwt_data(data: &mut [u8], freqs: &mut [u32], u8_index: &[usize]) {
    info!("        building burrow-wheeler-transform data ...\n");
    let mut j = 0;
    for i in 0..data.len() {
        while freqs[j] == 0 {
            j += 1
        }
        freqs[j] -= 1;
        data[u8_index[i]] = j as u8;
    }
}

/*---------------------------------------------*/
/*--- Fallback O(N log(N)^2) sorting        ---*/
/*--- algorithm, for repetitive blocks      ---*/
/*---------------------------------------------*/

/*---------------------------------------------*/

/// ADD DEPTH FACTOR
/// Sorts small bucket - could be slice between hi..lo if I reworked it. Caller must guarantee valid range.
fn fallback_simple_sort(u8_index: &mut [usize], data: &[u8], lo: usize, hi: usize, depth: usize) {
    if lo == hi {
        return;
    };
    if hi - lo > 3 {
        let mut i = hi - 4;
        while i >= lo {
            let tmp = u8_index[i];
            let ec_tmp = data[tmp];
            let mut j = i + 4;
            while j <= hi && ec_tmp > data[u8_index[j]] {
                u8_index[j - 4] = u8_index[j];
                j += 4;
            }
            u8_index[j - 4] = tmp;
            // Avoid underflow
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
    let mut i = hi - 1;
    while i >= lo {
        let tmp = u8_index[i];
        let ec_tmp = data[tmp];
        let mut j = i + 1;
        while j <= hi && ec_tmp > data[u8_index[j]] {
            u8_index[j - 1] = u8_index[j];
            j += 1;
        }
        u8_index[j - 1] = tmp;
        // Avoid underflow
        if i == 0 {
            break;
        }
        i -= 1;
    }
}

/// ADD DEPTH FACTOR
/// Three-way quick sort used by fallback_sort.
pub fn fallback_q_sort3(freq_map: &mut [usize], data: &mut [u8], lo_start: usize, hi_start: usize, depth:usize) {
    const FALLBACK_QSORT_SMALL_THRESH: usize = 10;
    const FALLBACK_QSORT_STACK_SIZE: usize = 100;
    let mut stack: Vec<(usize, usize)> = Vec::with_capacity(10);

    // Basic boundary test - don't waste time if there is nothing to sort!
    if hi_start - lo_start < 2 {
        return;
    }

    // C version uses this stack to push and pop ranges to sort. It MAY be
    // possible to just push and pop tuples to a vec, but this needs to be examined.
    stack.push((lo_start, hi_start));

    let mut random = 0;
    while !stack.is_empty() {
        // Error on excessive stack size
        if stack.len() >= FALLBACK_QSORT_STACK_SIZE - 1 {
            // ds: I want to see the error. I'm not sure how the stack can grow this much.
            error!(
                "Fallback_q_sort stack now {} over intended max size of {}. Error 1004",
                stack.len() - FALLBACK_QSORT_STACK_SIZE,
                FALLBACK_QSORT_STACK_SIZE
            );
            std::process::exit(1004)
        };

        // Set the range for this sub-slice sort.
        let (lo, hi) = stack.pop().unwrap();

        // Use a simpler quicksort if the slice between hi and lo is less than 10 (FALLBACK_QSORT_STACK_SIZE)
        if hi - lo < FALLBACK_QSORT_SMALL_THRESH {
            fallback_simple_sort(freq_map, data, lo, hi, depth);
            continue;
        }

        // // Check that we never try to sort past the end of the slice
        // assert!(freq_map[hi] < data.len());
        
        /* Julian's notes,  modified by ds:
        Random partitioning.  Selecting a pivot from a choice of 3 sometimes fails to avoid bad cases.
        A choice of 9 seems to help but looks rather expensive.
        Guidance for the magic constants 7621 and 32768 is
        taken from Sedgewick's algorithms book, chapter 35.
        */

        random = ((random * 7621) + 1) % 32768;
        let mut pivot = match random % 3 {
            0 => data[freq_map[lo]],
            1 => data[freq_map[(lo + hi) >> 1]],
            _ => data[freq_map[hi]],
        };

        // First sort lo-hi range into 3 parts: < pivot, == pivot and > pivot
        // Set the lower and upper moving boundries for where the three parts of the 3 way sort exist
        let mut lo_swap = lo;
        let mut hi_swap = hi - 1;
        let mut idx = lo;
        // Work through the slice swapping as needed until we hit the lower edge of the high boundary
        while idx <= hi_swap {
            match data[idx].cmp(&mut pivot) {
                // Elements below the pivot sort to the upper boundary of the low part
                std::cmp::Ordering::Less => {
                    data.swap(idx, lo_swap);
                    lo_swap += 1;
                    idx += 1;
                }
                std::cmp::Ordering::Equal => {
                    idx += 1;
                }
                // Elements above the pivot sort to the lower boundary of the upper part
                std::cmp::Ordering::Greater => {
                    data.swap(idx, hi_swap);
                    hi_swap -= 1;
                }
            }
        }
        /*
        Now that we have the three parts for the 3 way sort, push the both the low range and high range
        onto the stack and repeat the process until the stack is empty
         */
        if lo_swap > lo + 1 {
            stack.push((lo, lo_swap))
        }
        if hi > hi_swap + 1 {
            stack.push((hi_swap+1, hi))
        }
        println!("lo:{:6>}, lo_swap:{:6>}, hi_swap:{:6>}, hi:{:6>}.\r\x1B[1A", lo, lo_swap, hi_swap, hi);
    }
}
