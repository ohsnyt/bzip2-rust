use crate::{compression::compress::Block, tools::freq_count::freqs};
use log::{info, warn};

use super::fallback_q_sort3::fallback_q_sort3;

/// Sort function for blocks if the block size is less than 10k size or the block contains highly repetitive data.
pub fn fallback_sort(data: &[u8]) -> (u32, Vec<u8>) {
    // Julian shifted to 257 symbols. I'm trying to stay within 256.
    //    let mut data = data.iter().map(|b| *b as u16).collect::<Vec<u16>>();

    // Macros set/clear sentinal bits in the sentinels used in sorting.
    // There are 32 sentinal bits per bucket. Therefore shifting the index number right
    // by 5 bits will give us the bucket number, and the rightmost 5 bits will point to the
    // position within the bucket (1 << ($zz & 31)).
    macro_rules! set_sentinel {
        ($zz:expr) => {
            sentinels[$zz as usize >> 5] |= (1 << ($zz & 31))
        };
    }
    macro_rules! clear_sentinel {
        ($zz:expr) => {
            sentinels[$zz as usize >> 5] &= !(1 << ($zz & 31))
        };
    }
    macro_rules! is_set_sentinel {
        ($zz:expr) => {
            (sentinels[$zz as usize >> 5] & (1 << ($zz & 31)) as u32) > 0
        };
    }

    info!("     bucket sorting ...");
    // Create sentinel bits to mark the edges of all "buckets" for sorting.
    // Each bit in the sentinels vec refers to one u8 in the input data
    // Add one u32 to the vec for rounding up.
    // Add two u32s for block-end detection. (Not sure why Rust would need this.)
    let mut sentinels: Vec<u32> = vec![0_u32; 3 + (data.len() / 32)];

    // Build a frequency table and turn it into a cumulative sum table.
    let mut sum_freq = freqs(&data);
    sum_freq.iter_mut().fold(0, |acc, x| {
        *x += acc;
        *x
    });

    // Set a sentinel bit for each change in sum_freq.
    sum_freq.iter().for_each(|&el| set_sentinel!(el as usize));

    // Set mark/space sentinel bits at the end of the freq bits for block-end detection
    // (This sets a sequence of marks and space bits (101010...) after the regular sentinal bits.)
    (0..32).for_each(|i| {
        set_sentinel!(data.len() + 2 * i);
    });

    /*
    Now build an index (map of each u8 if sorted lexically) to each u8 in the input such that index[0]
    points to the first instance of the lexically lowest u8 in the input and index[index.len()] points
    to the last instance of the largest u8 in the input.
    */
    let mut u8_index =
        data.iter()
            .enumerate()
            .fold(vec![0_usize; data.len()], |mut map, (idx, byte)| {
                map[(sum_freq[*byte as usize] - 1) as usize] = idx;
                sum_freq[*byte as usize] -= 1;
                map
            });

    /*--
    Julian's note: Inductively refine the sentinels.  Kind-of an "exponential radix
    sort" (!), inspired by the  Manber-Myers suffix array construction algorithm.

    We will call the sort function log(N) times in this loop below. N will be determined by whether
    we still have "sentinels" that need to be sorted.

    We increase the number of characters to be considered each time by the power of 2.
    Set depth for the first iteration to 1 --*/
    let mut depth = 1;

    // Just to save rewriting datal.len()
    let end = data.len();

    loop {
        // Build a sort vec based on the depth level
        let mut sort_vec = u8_index.iter().enumerate().fold(
            (0_usize, vec![0_usize; end]),
            |(mut next_u8, mut vec), (i, el)| {
                if is_set_sentinel(i) {
                    next_u8 = i
                };
                vec[end + (u8_index[i] - depth) % end];
                (next_u8, vec)
            },
        );

        // Begin to count how many elements are not fully sorted
        // Initialize a counter to count how many unsorted elements exist at this level - used in reporting
        let mut not_done = 0;
        // Set the right boundary to -1 so we can initialize our index for the loop)
        let mut right: i32 = -1;
        loop {
            /*-- find the next non-singleton bucket boundry --*/
            let mut bndry = right + 1;

            // If bucket is not aligned to the u16 boundary, increment bucket so it is.
            while is_set_sentinel!(bndry) && (bndry&1==0) {
                bndry += 1;
            }
            if is_set_sentinel!(bndry) {
                // If bucket bndry indicates a bucket that has 32 members, increment bucket bndry by 32
                while (bndry >> 5) == 0xffffffff {
                    bndry += 32
                }
                // Otherwise increment by 1 by 1 in this loop until we find how big the bucket is
                while is_set_sentinel!(bndry) {
                    bndry += 1;
                }
            }

            // Set the "left" boundary of the bucket to sort
            let left = bndry - 1;
            if left >= data.len() as i32 {
                break;
            };
            // Look for the right boundary of the bucket. If we are not at a boundary and we
            // are unaligned, increment bucket
            while (sentinels[bndry as usize >> 5] & (1 << (bndry & 31)) as u32) == 0 {
                bndry += 1;
            }
            // If bucket bndry indicates a bucket that has 32 members, increment bndry by 32
            if (sentinels[bndry as usize >> 5] & (1 << (bndry & 31)) as u32) == 0 {
                bndry += 32;
            }
            // Otherwise increment by 1 by 1 in this loop until we find how big the bucket is
            while (sentinels[bndry as usize >> 5] & (1 << (bndry & 31)) as u32) == 0 {
                bndry += 1;
            }
            // Set the right boundary for the bucket
            right = bndry - 1;
            if right >= data.len() as i32 {
                break;
            }

            /*-- Julian's note: now [left, right] bracket current bucket --*/
            // Make sure left & right are logically sized
            if right > left {
                // The number of unsorted lines is incremented by the distance between left&right in this bucket
                not_done += right - left + 1;

                // Go sort this bucket (slice between left and right). This slice will always be 32 or less bytes.
                // (Only sort if needed, though.)
                if not_done > 1 && left >= 0 && left < right {
                    fallback_q_sort3(&mut u8_index, &data, left, right)
                }

                /*--
                Scan bucket we just sorted and generate new header bits.
                NOTE: A set header bit indicates the start of a new bucket. Therefore we never
                need to clear header bits - when they are either all set, or all 1010, we are done.
                -- */
                let mut cc: i32 = -1;
                for i in left..=right {
                    let cc1 = data[u8_index[i as usize] as usize] as i32;
                    if cc != cc1 {
                        set_sentinel!(i);
                        cc = cc1;
                    };
                }
            }
        }

        info!(
            "depth {:>7} has {} unresolved strings",
            depth, not_done
        );

        //depth *= 2;
        depth <<= 1;
        if depth > data.len() as i32 || not_done == 0 {
            break;
        };
    }

    // Generate the burrow-wheeler data.
    info!("        building burrow-wheeler-transform data ...\n");
    let key: u32;
    let mut bwt_data = vec![0; data.len()];
    for i in 0..data.len() {
        if u8_index[i] == 0 {
            key = i as u32;
            bwt_data[i] = data[data.len() - 1] as u8;
        } else {
            bwt_data[i] = data[u8_index[i] as usize - 1] as u8
        }
    }
    // Return the key and data
    (key, data)
}

/*---------------------------------------------*/
/*--- Fallback O(N log(N)^2) sorting        ---*/
/*--- algorithm, for repetitive blocks      ---*/
/*---------------------------------------------*/

/*---------------------------------------------*/

/// Sorts small bucket - could be slice between hi..lo if I reworked it.
pub fn fallback_simple_sort(u8_index: &mut [u32], data: &[u16], lo: i32, hi: i32, _h: i32) {
    if lo == hi {
        return;
    };
    if hi - lo > 3 {
        let mut i = hi - 4;
        while i >= lo {
            let tmp = u8_index[i as usize];
            let ec_tmp = data[tmp as usize];
            let mut j = i + 4;
            while j <= hi && ec_tmp > data[u8_index[j as usize] as usize] {
                u8_index[j as usize - 4] = u8_index[j as usize];
                j += 4;
            }
            u8_index[j as usize - 4] = tmp;
            i -= 1;
        }
    }
    let mut i = hi - 1;
    while i >= lo {
        let tmp = u8_index[i as usize];
        let ec_tmp = data[tmp as usize];
        let mut j = i + 1;
        while j <= hi && ec_tmp > data[u8_index[j as usize] as usize] {
            u8_index[j as usize - 1] = u8_index[j as usize];
            j += 1;
        }
        u8_index[j as usize - 1] = tmp;
        i -= 1;
    }
}

/// Sorts slice using Rust's fast .sort_unstable_by
pub fn fallback_simple_sort2(map_slice: &mut [u32], data: &[u16]) {
    map_slice.sort_unstable_by(|a, b| data[*a as usize].cmp(&data[*b as usize]));
}
