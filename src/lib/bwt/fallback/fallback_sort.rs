
use log::info;
use crate::lib::compress::Block;

use super::fallback_q_sort3::fallback_q_sort3;

/// Sort function for blocks of less than 10k size and highly repetitive data.
pub fn fallback_sort(block: &mut Block)  {

    // This algorithm actually needs to use 257 distinct symbols, so we need to convert
    // the input to a u16 format.
    let mut block_data = block.data.iter().map(|b| *b as u16).collect::<Vec<u16>>();

    // Create and initialize vecs for the transformed data and frequency tables
    let mut bhtab: Vec<u32> = vec![0_u32; 4 + (block.end / 32)];
    let mut freq_map = vec![0_u32; block.end];

    /*
    bhtab sets the bucket tables for the radix sorting algorithm.

    The 5 bit shift does this: We can multilple store "sentinels" (bucket edges) in each u32.
    There are 256 different u8 symbols. Each u32 here allows for defining 32 of those symbols.
    This means we should at max see 8 entries in this table. Julian says we can have 2+block.end/32, or
    a max of 10 entries.

    These are built in three places:
    1: from the sum_freq table
    2: to set sentinel bits for block-block.end detection (together with clear_bh)
        for i in 0..32 { set_bh!(block.end + 2 * i); clear_bh!(block.end + 2 * i + 1); }
    3: to scan each processed bucket and generate header bits that will indicate every time
        within a bucket when a new sort group is found

    Point 3 is related to radix sorting. For example, all the "a's" get sorted together. Then
    within that "bucket" we sort by the next character. Every "a" that shares the same next
    character gets put into a shared sub-bucket for the next go round of sorting.
    */
    macro_rules! set_bh {
        ($zz:expr) => {
            bhtab[$zz as usize >> 5] |= (1 << ($zz & 31));
        };
    }
    macro_rules! clear_bh {
        ($zz:expr) => {
            bhtab[$zz as usize >> 5] &= !(1 << ($zz & 31));
        };
    }
    macro_rules! is_set_bh {
        ($zz:expr) => {
            (bhtab[$zz as usize >> 5] & (1 << ($zz & 31)) as u32) > 0
        };
    }
    // I'm not sure how word_bh fits in.
    macro_rules! word_bh {
        ($zz:expr) => {
            bhtab[$zz as usize >> 5]
        };
    }
    // This has to do with Julian's use of u32 indexing intermingled with u8 indexing.
    // The u32 indexing required word alignment to work.
    // I'm not sure it is still relevant in the Rust version. ds
    macro_rules! unaligned_bh {
        ($zz:expr) => {
            $zz & 0x1f != 0
        };
    }

    info!("     bucket sorting ...");
    // Build frequency table
    let freq = block_data.iter().fold(vec![0_u32; 256], |mut v, b| {
        v[*b as usize] += 1;
        v
    });

    // Create a cumulative sum frequency table
    let (mut sum_freq, _) =
        freq.iter()
            .enumerate()
            .fold((vec![0_u32; 256], 0), |(mut v, mut sum), (i, f)| {
                sum += f;
                v[i] = sum;
                (v, sum)
            });
    // sum_freq (ftab) needs to be one entry longer to work with loops below.
    sum_freq.push(sum_freq[sum_freq.len() - 1]);

    /*
    Now build the map/index. This requires us to read a byte from the input, look it up in the
    sum_freq table, reduce the table by the one we found, and put the index sequence into the map.
    */
    for (idx, byte) in block_data.iter().enumerate() {
        let tmp = sum_freq[*byte as usize] - 1;
        sum_freq[*byte as usize] = tmp;
        freq_map[tmp as usize] = idx as u32;
    }

    // Set a count-change marker for each change in sum_freq.
    for i in 0..256_usize {
        set_bh!(sum_freq[i]);
    }

    /*--
    Set sentinel bits (bh = header bits / bit headers) for block-block.end detection.
    (This sets a sequence of marks and space bits (101010...) in bhtab 3, 4 and 5.) --*/
    for i in 0..32 {
        set_bh!(block.end + 2 * i);
        clear_bh!(block.end + 2 * i + 1);
    }

    /*--
    Julian's note: Inductively refine the buckets.  Kind-of an "exponential radix
    sort" (!), inspired by the  Manber-Myers suffix array construction algorithm.

    We will call the sort function log(N) times in this loop below. N will be determined by whether
    we still have "buckets" that need to be sorted.
    We increase the number of characters to be considered each time by the power of 2.
    Set depth for the first iteration to 1 --*/
    let mut depth = 1;

    loop {
        let mut j = 0;

        // Iterate through every byte of the input data and update the input data
        for i in 0..block.end {
            // If a count-change marker is set for this index number, note this index number
            if is_set_bh!(i) {
                j = i
            };
            /*
            Get the offset in freq_map associated with this iteration, and subtract the loop level (depth).
            What this should do is point to the byte previous to this byte (previous by the loop level)
            */
            let mut offset = freq_map[i as usize] as i32 - depth;
            // if this offset is less than zero, wrap around the block.end of the input string
            if offset < 0 {
                offset += block.end as i32;
            };
            /*
            Update the input data at the offset we calculated to be equal to the bucket transition index
            number we noted above. Every byte in the input will now have a new value that reflects the
            the bucket of the following byte!

            Thus banana will be 040403, where the 0 indicates the next byte belongs to bucket 0, the 4 indicates
            the next byte belongs to bucket 4, etc.
            */
            block_data[offset as usize] = j as u16;
        }

        // Begin to count how many lines are not fully sorted
        // Initialize a counter to count how many unsorted lines exist at this level - used in reporting
        let mut not_done_count = 0;
        // Set the right boundary to -1 so we can initialize our index for the loop)
        let mut right: i32 = -1;
        loop {
            /*-- find the next non-singleton bucket boundry --*/
            let mut bndry = right + 1;

            // If bucket is not aligned to the u16 boundary, increment bucket so it is.
            while is_set_bh!(bndry) && unaligned_bh!(bndry) {
                bndry += 1;
            }
            if is_set_bh!(bndry) {
                // If bucket bndry indicates a bucket that has 32 members, increment bucket bndry by 32
                while word_bh!(bndry) == 0xffffffff {
                    bndry += 32
                }
                // Otherwise increment by 1 by 1 in this loop until we find how big the bucket is
                while is_set_bh!(bndry) {
                    bndry += 1;
                }
            }

            // Set the "left" boundary of the bucket to sort
            let left = bndry - 1;
            if left >= block.end as i32 {
                break;
            };
            // Look for the right boundary of the bucket. If we are not at a boundary and we
            // are unaligned, increment bucket
            while (bhtab[bndry as usize >> 5] & (1 << (bndry & 31)) as u32) == 0 {
                bndry += 1;
            }
            // If bucket bndry indicates a bucket that has 32 members, increment bndry by 32
            if (bhtab[bndry as usize >> 5] & (1 << (bndry & 31)) as u32) == 0 {
                    bndry += 32;
                }
            // Otherwise increment by 1 by 1 in this loop until we find how big the bucket is
            while (bhtab[bndry as usize >> 5] & (1 << (bndry & 31)) as u32) == 0 {
                bndry += 1;
            }
            // Set the right boundary for the bucket
            right = bndry - 1;
            if right >= block.end as i32 {
                break;
            }

            /*-- Julian's note: now [left, right] bracket current bucket --*/
            // Make sure left & right are logically sized
            if right > left {
                // The number of unsorted lines is incremented by the distance between left&right in this bucket
                not_done_count += right - left + 1;

                // Go sort this bucket (slice between left and right). This slice will always be 32 or less bytes.
                // (Only sort if needed, though.)
                if not_done_count > 1 && left >= 0 && left < right {
                    fallback_q_sort3(&mut freq_map, &block_data, left, right)
                }

                /*--
                Scan bucket we just sorted and generate new header bits.
                NOTE: A set header bit indicates the start of a new bucket. Therefore we never
                need to clear header bits - when they are either all set, or all 1010, we are done.
                -- */
                let mut cc: i32 = -1;
                for i in left..=right {
                    let cc1 = block_data[freq_map[i as usize] as usize] as i32;
                    if cc != cc1 {
                        set_bh!(i);
                        cc = cc1;
                    };
                }
            }
        }

        info!(
            "depth {}{} has {} unresolved strings",
            if depth < 10 { " " } else { "" },
            depth,
            not_done_count
        );

        depth *= 2;
        if depth > block.end as i32 || not_done_count == 0 {
            break;
        };
    }

    // Generate the burrow-wheeler data.
    info!("        building burrow-wheeler-transform data ...\n");
    let mut bwt_data = vec![0; block.end];
    for i in 0..block.end as usize {
        if freq_map[i] == 0 {
            block.key = i;
            bwt_data[i] = block.data[block.end - 1] as u8;
        } else {
            bwt_data[i] = block.data[freq_map[i] as usize - 1] as u8
        }
    }
        // Shift ownership of bwt_data to block.data
    block.data.clear();
    block.data = bwt_data;

}
