use log::{error, info};

use crate::{compression::compress::Block, tools::freq_count::freqs};

/// Fallback sort function for small blocks or block which contains highly repetitive data.
/// Rewritten by ds.
pub fn fallback_sort_ds(block: &mut Block) {
    log::debug!("Using fallback_sort_ds");
    /*
    Julian noted that this is is a kind-of an "exponential radix sort" inspired by the
    Manber-Myers suffix array construction algorithm. We will call the sort function log(N)
    times in this loop below. N will be determined by whether we still have "sentinels"
    that need to be sorted. We increase the number of characters to be considered each time
    by the power of 2.
    */

    // Create simple reference to the length of the input vec, for readability...
    let end = block.data.len();

    // Build a frequency table (we need it at the end to rebuild data) and use it to make a cumulative sum table.
    let mut freqs = freqs(&block.data);
    let mut sum_freq = freqs.clone();
    sum_freq.iter_mut().fold(0, |acc, x| {
        *x += acc;
        *x
    });

    /*
    Now build an index (map of each u8 if sorted lexically) to each u8 in the input such that index[0]
    points to the first instance of the lexically lowest u8 in the input and index[index.len()] points
    to the last instance of the largest u8 in the input. This will be progressively sorted to achive
    the final BWT index map.
    */
    let mut index =
        block
            .data
            .iter()
            .enumerate()
            .fold(vec![0_usize; end], |mut map, (idx, byte)| {
                map[(sum_freq[*byte as usize] - 1) as usize] = idx;
                sum_freq[*byte as usize] -= 1;
                map
            });

    /*
    To track which elements of the input data are sorted/unsorted, we use sentinel bits for each
    element. Each bit in the sentinels vec refers to one u8 in the input data. A set bit indicates
    that the element is fully sorted. A zero bit indicates it is not sorted.

    We need to initialize the sentinel vec before declaring the macros that refer to them. We need
    one bit for each element, rounded up to the next largest u32.
    */
    let mut sentinels: Vec<u32> = vec![0_u32; 1 + (end / 32)];

    /*
    The macros here set/clear sentinal bits used to keep track of which elements are fully sorted.
    There are 32 sentinal bits per bucket (32 = 2^5). Therefore shifting the index number right
    by 5 bits will give us the bucket number, and the rightmost 5 bits will point to the
    position within the bucket (1 << ($zz & 31)).
    */
    /// Remember that this element is fully sorted
    macro_rules! set_sentinel {
        ($zz:expr) => {
            sentinels[$zz as usize >> 5] |= (1 << ($zz & 31))
        };
    }
    /// Report if this element is fully sorted
    macro_rules! is_set_sentinel {
        ($zz:expr) => {
            (sentinels[$zz as usize >> 5] & (1 << ($zz & 31)) as u32) > 0
        };
    }
    /// Report if this element is NOT fully sorted
    macro_rules! is_not_set_sentinel {
        ($zz:expr) => {
            (sentinels[$zz as usize >> 5] & (1 << ($zz & 31)) as u32) <= 0
        };
    }

    // Fill the initial sentinel vec based on the sum_freq vector.
    sum_freq.iter().for_each(|&el| set_sentinel!(el as usize));

    // Set the initial depth of sorting
    let mut depth = 1;

    info!("     bucket sorting ...");
    loop {
        let mut j = 0;

        for i in 0..end {
            if is_set_sentinel!(i) {
                j = i
            }
            let k = (end + index[i] - depth) % end;
            block.data[k] = j as u8;
        }

        // Track how many unsorted elements exist at this level
        let mut unsorted_found = 0;

        // Look for sequences of unsorted elements. First initialize left boundary index
        let mut left = 0;
        loop {
            // Find the next left position. If no left, break the loop
            while (is_set_sentinel!(left)) && ((left & 0x1f) == 0) {
                left += 1
            }

            if is_set_sentinel!(left) {
                while sentinels[left >> 5] == 0xffffffff {
                    left += 32
                }
                while is_set_sentinel!(left) {
                    left += 1
                }
            }
            left -= 1;
            if left >= end {
                break;
            }

            // Not initialize and find the right boundary
            let mut right = left + 1;
            while is_not_set_sentinel!(right) && ((right & 0x1f) == 0) {
                right += 1;
            }
            if is_not_set_sentinel!(right) {
                while sentinels[right >> 5] == 0x0 {
                    right += 32
                }
                while is_not_set_sentinel!(right) {
                    right += 1
                }

                if right >= end {
                    break;
                }
            }

            // Indicate how much work has been done.
            unsorted_found += right - left;

            // The above should only result in valid slices greater than 1 element long. Go sort it.
            fallback_q_sort3(&mut index, &block.data, left, right);

            /*-- Scan the now sorted bucket to mark those element that were fully sorted-- */
            let mut compare = block.data[index[left]];
            set_sentinel!(left);

            for i in left + 1..right {
                if compare != block.data[index[i]] {
                    set_sentinel!(i);
                    compare = block.data[index[i]];
                }
            }

            // Go on to the next unsorted slice in this data.
            left = right;
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

    info!("        building burrow-wheeler-transform data ...\n");
    // Rebuild the original data so we can generate the output.
    let mut data = vec![0; end];
    let mut j = 0;
    index.iter().for_each(|&idx| {
        while freqs[j] == 0 {
            j += 1
        }
        freqs[j] -= 1;
        data[idx] = j as u16
    });

    // Generate key and put the BWT output into block.data
    index.iter().enumerate().for_each(|(i, &el)| {
        if el == 0 {
            block.key = i as u32;
            block.data[i] = data[end - 1] as u8
        } else {
            block.data[i] = data[el - 1] as u8
        };
    });

    // clear the rle2 data vec
    block.rle2.clear();
}

/*---------------------------------------------*/
/*--- Fallback O(N log(N)^2) sorting        ---*/
/*--- algorithm, for repetitive blocks      ---*/
/*---------------------------------------------*/

/*---------------------------------------------*/
/*
/// Using built-in sort_unstable instead of this
fn fallback_simple_sort(index: &mut [usize], data: &[u8], lo: usize, hi: usize) {
    if lo == hi {
        return;
    };
    if hi - lo > 3 {
        let mut i = hi - 4;
        while i >= lo {
            let tmp = index[i];
            let ec_tmp = data[tmp];
            let mut j = i + 4;
            while j <= hi && ec_tmp > data[index[j]] {
                index[j - 4] = index[j];
                j += 4;
            }
            index[j - 4] = tmp;
            // Avoid underflow
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
    let mut i = hi - 1;
    while i >= lo {
        let tmp = index[i];
        let ec_tmp = data[tmp];
        let mut j = i + 1;
        while j <= hi && ec_tmp > data[index[j]] {
            index[j - 1] = index[j];
            j += 1;
        }
        index[j - 1] = tmp;
        // Avoid underflow
        if i == 0 {
            break;
        }
        i -= 1;
    }
}
 */
/// Three-way quick sort used by fallback_sort.
pub fn fallback_q_sort3(index: &mut [usize], data: &[u8], lo_start: usize, hi_start: usize) {
    const FALLBACK_QSORT_SMALL_THRESH: usize = 10;
    const FALLBACK_QSORT_STACK_SIZE: usize = 100;
    let mut stack: Vec<(usize, usize)> = Vec::with_capacity(10);

    // Basic boundary test - don't waste time if there is nothing to sort!
    if hi_start - lo_start < 2 {
        return;
    }

    // Push the next range onto the stack. After we subdivide this slice into "smaller than pivot"
    // and "greater than pivot", those slices will go on the stack if they contain more than 1 element.
    stack.push((lo_start, hi_start));

    let mut random = 0;
    while !stack.is_empty() {
        // Error on excessive stack size
        if stack.len() >= FALLBACK_QSORT_STACK_SIZE {
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

        // Use simple quicksort if the slice is less than 10 elements (FALLBACK_QSORT_SMALL_THRESH)
        if hi - lo < FALLBACK_QSORT_SMALL_THRESH {
            //fallback_simple_sort(freq_map, data, lo, hi); // use builtin quicksort instead
            index[lo..hi].sort_unstable_by(|&a, &b| data[a].cmp(&data[b]));
            continue;
        }

        // More complex 3 way quick sort for larger slices is here -----------------

        /* Julian's notes,  modified by ds:
        Rather than just using the last element as the pivot, Julian "randomly" chooses a
        pivot from a choice of 3. Yet sometimes even this fails to avoid bad cases.
        Guidance for the magic constants 7621 and 32768 is taken from Sedgewick's algorithms
        book, chapter 35.
        */

        // Recalculate random and get a pivot
        random = ((random * 7621) + 1) % 32768;
        let mut pivot = match random % 3 {
            0 => data[index[lo]],
            1 => data[index[(lo + hi - 1) >> 1]],
            _ => data[index[hi - 1]],
        };

        // Set the lower and upper swap positions for the 3 way sort
        let mut lo_swap = lo;
        let mut hi_swap = hi - 1;
        let mut idx = lo;

        // Loop through the slice until until we have looked at every element in the slice.
        // We may move the high swap point down. No need to loop past that point.
        while idx < hi_swap + 1 {
            // Compare the data element at the current index with the pivot
            match data[index[idx]].cmp(&mut pivot) {
                // Elements less than the pivot swap down to lo_swap position
                std::cmp::Ordering::Less => {
                    if data[index[idx]] < data[index[lo_swap]] {
                        index.swap(idx, lo_swap);
                    }
                    lo_swap += 1;
                    idx += 1;
                }
                std::cmp::Ordering::Equal => {
                    idx += 1;
                }
                // Elements greater than the pivot sort up to hi_swap position, but first
                // compare the element at hi_swap to the pivot
                std::cmp::Ordering::Greater => match data[index[hi_swap]].cmp(&pivot) {
                    // If hi_swap is less than the pivot, just swap and go check the pivot again
                    // (It may need to swap down to the lo_swap position)
                    std::cmp::Ordering::Less => {
                        index.swap(idx, hi_swap);
                    }
                    // If hi_swap is equal to the pivot, swap the element at hi_swap down to the pivot
                    // and adjust idx and the hi_swap locations
                    std::cmp::Ordering::Equal => {
                        index.swap(idx, hi_swap);
                        hi_swap -= 1;
                        idx += 1;
                    }
                    // If hi_swap is greater than the pivot, move hi_swap down until it isn't
                    // This avoids unnecessary swapping
                    std::cmp::Ordering::Greater => {
                        while data[index[hi_swap]] > pivot {
                            hi_swap -= 1
                        }
                    }
                },
            }
        }

        /*
        Now that we have the three parts for the 3 way sort, push the both the low range and high range
        onto the stack and repeat the process until the stack is empty
         */

        if lo_swap > 1 + lo {
            stack.push((lo, lo_swap))
        }
        if hi > hi_swap + 2 {
            stack.push((hi_swap + 1, hi))
        }
    }
}
