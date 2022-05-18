use log::{debug, error, info};

use super::main_q_sort3::main_q_sort3;

const MAIN_QSORT_STACK_SIZE: usize = 100;
const BZ_N_RADIX: i32 = 2;

pub struct QSort {
    pub stack: Vec<(i32, i32, i32)>,
    pub end: usize,
}

impl QSort {
    fn new() -> Self {
        Self {
            stack: Vec::with_capacity(MAIN_QSORT_STACK_SIZE),
            end: 0,
        }
    }
}

pub fn main_sort(block_data8: &[u8], mut budget: i32) -> (i32, usize, Vec<u8>) {
    info!("Main sort initialize.");

    // initialize key constants
    const OVERSHOOT: usize = 34;
    let end = block_data8.len();
    let mut qs = QSort::new();
    qs.end = block_data8.len();

    // Initialize data structures
    let mut quadrant: Vec<u16> = vec![0; end + OVERSHOOT];
    let mut copy_start = vec![0_i32; 256];
    let mut copy_end = vec![0_i32; 256];

    // We need to convert the input to a u16 format
    let mut block_data = block_data8.iter().map(|b| *b as u16).collect::<Vec<u16>>();
    // And wrap the beginning data around OVERSHOOT length at the end.
    block_data.extend(
        block_data8[0..OVERSHOOT]
            .iter()
            .map(|b| *b as u16)
            .collect::<Vec<u16>>(),
    );

    // Build the two-byte freq_tab table
    // NOTE, Julian does this in blocks of 4 because loops are slower than sequential code.
    // Rust optimizes this automatically. Iter_mut is slightly faster than either for loop.
    let mut j = (block_data8[0] as u16) << 8;
    let mut freq_tab = block_data8
        .iter()
        .rev()
        .fold(vec![0_u32; 65536 + 1], |mut vec, byte| {
            j = (j >> 8) | (*byte as u16) << 8;
            vec[j as usize] += 1;
            vec
        });

    // Turn the freq_tab count into a cumulative sum of freq_tab. Iter_mut is 4x faster than for loop
    freq_tab.iter_mut().fold(0, |acc, x| {
        *x += acc;
        *x
    });

    info!("   bucket sorting ...");

    // Create a two-byte data vec from the block array (UInt8 array) so we can work against 2 bytes at once
    let mut s = (block_data[0]) << 8;

    let mut bwt_ptr = block_data.iter().enumerate().rev().skip(OVERSHOOT).fold(
        vec![0; block_data.len()],
        |mut vec, (idx, byte)| {
            s = (s >> 8) | (*byte) << 8;
            let j = (freq_tab[s as usize]) - 1;
            freq_tab[s as usize] = j;
            vec[j as usize] = idx as u32;
            vec
        },
    );

    // Initialize big_done
    let mut big_done = vec![false; 256];
    // Initialize running_order as a vec with values from 0 - 255
    let mut running_order = (0..=255_u8).fold(vec![], |mut v: Vec<u8>, n| {
        v.push(n);
        v
    });

    // Why not just set h to 364????
    // let mut h = 1;
    // while h <= 256 {
    //     h = 3 * h + 1;
    // }
    let mut h = 364;

    // Do a rough, partial sort of running_order based on data in big_freq
    // running_order is the "big bucket" in which the little buckets reside
    while h != 1 {
        h /= 3;
        for i in h..=255 {
            let vv = running_order[i] as usize;
            let mut j = i;
            'outer: while big_freq(&freq_tab, running_order[(j - h) as usize] as u32)
                > big_freq(&freq_tab, vv as u32)
            {
                running_order[j] = running_order[j - h];
                j -= h;
                if j <= (h - 1) {
                    break 'outer;
                }
            }
            running_order[j] = vv as u8;
        }
    }
    // The main sorting loop
    // Initialize how many "rows" have been quick sorted
    let mut num_q_sorted = 0;
    /*--
       Process big buckets, starting with the least full.
       Basically this is a 3-step process in which we call
       mainQSort3 to sort the small buckets [ss, j], but
       also make a big effort to avoid the calls if we can.
    --*/
    for (i, &ss) in running_order.iter().enumerate() {
        /*--
           Step 1:
           Complete the big bucket [ss] by quicksorting
           any unsorted small buckets [ss, j], for j != ss.
           Hopefully previous pointer-scanning phases have already
           completed many of the small buckets [ss, j], so
           we don't have to sort them at all.
        --*/
        const SETMASK: u32 = 1 << 21;
        const CLEARMASK: u32 = !SETMASK;

        for j in 0..=255 {
            if j != ss {
                // This moves ss into the second byte of sb, and j into the first.
                // First time through, freq_tab has nothing in the upper 2 bytes, so this
                // will call main_q_sort3 every time that freq_tab[sb+1] > freq_tab[sb].
                let sb = ((ss as u32) << 8) + j as u32;
                if 0 == (freq_tab[sb as usize] & SETMASK) {
                    let lo = (freq_tab[sb as usize] & CLEARMASK) as i32;
                    let hi = (freq_tab[sb as usize + 1] & CLEARMASK) as i32 - 1;
                    if hi > lo {
                        qs.stack.clear();
                        qs.stack.push((lo, hi, BZ_N_RADIX));
                        // Report progress
                        debug!(
                            "   qsort [0x{:0x}, 0x{:0x}]   done {}   this {}",
                            ss,
                            j,
                            num_q_sorted,
                            hi - lo + 1
                        );
                        // Then sort the bucket
                        main_q_sort3(
                            &mut bwt_ptr,
                            &block_data,
                            &mut quadrant,
                            &mut budget,
                            &mut qs,
                        );
                        // Update our count of rows that are now sorted
                        num_q_sorted += hi - lo + 1;

                        // if the sorting was too "expensive", we fail out and try the fallback method
                        if budget < 0 {
                            // let mut bwt_data: Vec<u8> = vec![0; end];
                            // let mut key = 0;
                            // for i in 0..end as usize {
                            //     if bwt_ptr[i] == 0 {
                            //         bwt_data[i] = block_data8[end - 1] as u8;
                            //         key = i;
                            //     } else {
                            //         bwt_data[i] = block_data8[bwt_ptr[i] as usize - 1] as u8
                            //     }
                            // }
                            let bwt_data: Vec<u8> = vec![];
                            let key = 0;
                            return (budget, key, bwt_data);
                        };
                    }
                }
                freq_tab[sb as usize] |= SETMASK;
            }
        }
        /*--
         Step 2:
         Now scan this big bucket [ss] so as to synthesise the
         sorted order for small buckets [t, ss] for all t,
         including, magically, the bucket [ss,ss] too.
         This will avoid doing Real Work in subsequent Step 1's.
        --*/

        // Since copy_start and copy_end are fully overwritten, no need to initialize them
        // Set bucket start and end marks
        (0..256).for_each(|i| {
            let idx = (i << 8) + ss as usize;
            copy_start[i] = (freq_tab[idx] & CLEARMASK) as i32;
            copy_end[i] = (freq_tab[idx + 1] & CLEARMASK) as i32 - 1;
        });

        {
            let mut j = (freq_tab[(ss as usize) << 8] & CLEARMASK) as i32;
            while  j < copy_start[ss as usize] {
                let mut k = bwt_ptr[j as usize] as i32 - 1;
                let mut k = bwt_ptr[j as usize] as i32 - 1;
                if k < 0 {
                    k += end as i32;
                };
                let c1 = block_data[k as usize];
                if !big_done[c1 as usize] {
                    bwt_ptr[copy_start[c1 as usize] as usize] = k as u32;
                    copy_start[c1 as usize] += 1;
                }
                j += 1;
            }
            let mut j = ((freq_tab[(ss as usize + 1) << 8] & CLEARMASK) as i32) - 1;
            while j > copy_end[ss as usize] {
                let mut k = bwt_ptr[j as usize] as i32 - 1;
                if k < 0 {
                    k += end as i32
                }
                let c1 = block_data[k as usize];

                if !big_done[c1 as usize] {
                    bwt_ptr[copy_end[c1 as usize] as usize] = k as u32;
                    copy_end[c1 as usize] -= 1;
                }
                j -= 1;
            }
        }
        /*
        Extremely rare case missing in bzip2-1.0.0 and 1.0.1.
        Necessity for this case is demonstrated by compressing a sequence of approximately
        48.5 million of character 251; 1.0.0/1.0.1 will then die here.
        */
        if (copy_start[ss as usize] - 1 != copy_end[ss as usize])
            || ((copy_start[ss as usize] == 0) && copy_end[ss as usize] == end as i32 - 1)
        {
            error!("Massive 251 attack detected!")
        }

        for j in 0..256_usize {
            freq_tab[(j << 8) + ss as usize] |= SETMASK;
        }

        /*--
         Step 3:
         The [ss] big bucket is now done.  Record this fact,
         and update the quadrant descriptors.  Remember to
         update quadrants in the overshoot area too, if
         necessary.  The "if (i < 255)" test merely skips
         this updating for the last bucket processed, since
         updating for the last bucket is pointless.

         The quadrant array provides a way to incrementally
         cache sort orderings, as they appear, so as to
         make subsequent comparisons in fullGtU() complete
         faster.  For repetitive blocks this makes a big
         difference (but not big enough to be able to avoid
         the fallback sorting mechanism, exponential radix sort).

         The precise meaning is: at all times:

            for 0 <= i < nblock and 0 <= j <= nblock

            if block[i] != block[j],

               then the relative values of quadrant[i] and
                    quadrant[j] are meaningless.

               else {
                  if quadrant[i] < quadrant[j]
                     then the string starting at i lexicographically
                     precedes the string starting at j

                  else if quadrant[i] > quadrant[j]
                     then the string starting at j lexicographically
                     precedes the string starting at i

                  else
                     the relative ordering of the strings starting
                     at i and j has not yet been determined.
               }
        --*/
        big_done[ss as usize] = true;

        if i < 255 {
            let bb_start = (freq_tab[(ss as usize) << 8] & CLEARMASK) as i32;
            let bb_size = ((freq_tab[(ss as usize + 1) << 8] & CLEARMASK) as i32) - bb_start;
            let mut shifts: u32 = 0;

            while (bb_size >> shifts) > 65534 {
                shifts += 1;
            }

            let mut j = bb_size - 1;
            while j >= 0 {
                let a2update = bwt_ptr[bb_start as usize + j as usize] as usize;
                let q_val = (j as u16) >> shifts;
                quadrant[a2update] = q_val;
                if a2update < OVERSHOOT {
                    quadrant[a2update + end] = q_val
                }
                j -= 1;
            }
            if (bb_size - 1) >> shifts > 65535 {
                error!("Shifted too many times during BWT sort")
            };
        }
    }
    info!(
        "{} pointers, {} sorted, {} scanned",
        end,
        num_q_sorted,
        end as i32 - num_q_sorted
    );

    info!("        building burrow-wheeler-transform data ...\n");
    let mut bwt_data = vec![0; end];
    let mut key = 0;
    for i in 0..end as usize {
        if bwt_ptr[i] == 0 {
            key = i;
            bwt_data[i] = block_data8[end - 1] as u8;
        } else {
            bwt_data[i] = block_data8[bwt_ptr[i] as usize - 1] as u8
        }
    }
    (budget, key, bwt_data)
}

/// Return the difference between the second bytes in freq_tab[[n+1]] and freq_tab[[n]].
/// The difference is returned as the second byte in a u32 (00_0).
#[inline(always)]
fn big_freq(freq_tab: &[u32], n: u32) -> u32 {
    (freq_tab[((n + 1) as usize) << 8] as u32) - (freq_tab[(n as usize) << 8] as u32)
}
