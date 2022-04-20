use log::{error, info};

use super::main_q_sort3::main_q_sort3;

pub fn main_sort(block_data8: &[u8], mut budget: i32) -> (i32, usize, Vec<u8>) {
    info!("Main sort initialize.");
    //debug
    //let watch_me = 121;

    // initialize key constants
    const OVERSHOOT: usize = 34;
    let end = block_data8.len();

    // Initialize data structures, including a 2 byte frequency table
    let mut freq_tab: Vec<u32> = vec![0; 65536 + 1];
    let mut quadrant: Vec<u16> = vec![0; end + OVERSHOOT];
    let mut bwt_ptr: Vec<u32> = vec![0; end + OVERSHOOT];

    // We need to convert the input to a u16 format and wrap around OVERSHOOT length.
    let mut block_data = block_data8.iter().map(|b| *b as u16).collect::<Vec<u16>>();
    block_data.extend(
        block_data8[0..OVERSHOOT]
            .iter()
            .map(|b| *b as u16)
            .collect::<Vec<u16>>(),
    );

    // Build the two-byte freq_tab table
    // NOTE, Julian does this in blocks of 4, presumabily because loops are slower than sequential code.
    let mut j = (block_data[0] as u16) << 8;
    for i in (0..end).rev() {
        j = (j >> 8) | (block_data[i] as u16) << 8;
        freq_tab[j as usize] += 1;
    }

    info!("   bucket sorting ...");
    // Turn the freq_tab count into a cumulative sum of freq_tab
    for i in 1..freq_tab.len() {
        freq_tab[i] += freq_tab[i - 1]
    }

    // I think... copy the byte data from the block array (UInt8 array) into word data in s array (UInt16 array)
    // Trying to simplify - does it slow it down to iterate??
    let mut s = (block_data[0] as u16) << 8;
    // Skipping the 4x iteration for now - just use the simple loop indexing by one
    //let mut i = end - 1;

    //  while i >= 3 {
    //     s = (s >> 8) | (block_data[i] as u16) << 8;
    //     j = (freq_tab[s as usize] as u16) - 1;
    //     freq_tab[s as usize] = j as u32;
    //     bwt_ptr[j as usize] = i as u32;
    //     s = (s >> 8) | (block_data[i - 1] as u16) << 8;
    //     j = (freq_tab[s as usize] as u16) - 1;
    //     freq_tab[s as usize] = j as u32;
    //     bwt_ptr[j as usize] = (i - 1) as u32;
    //     s = (s >> 8) | (block_data[i - 2] as u16) << 8;
    //     j = (freq_tab[s as usize] as u16) - 1;
    //     freq_tab[s as usize] = j as u32;
    //     bwt_ptr[j as usize] = (i - 2) as u32;
    //     s = (s >> 8) | (block_data[i - 3] as u16) << 8;
    //     if freq_tab[s as usize ] < 1 {
    //         println!("Pause")
    //     }
    //     j = (freq_tab[s as usize] as u16) - 1;
    //     freq_tab[s as usize] = j as u32;
    //     bwt_ptr[j as usize] = (i - 3) as u32;

    //     i -= 4;
    // }
    // I need j to be u32 now.
    let mut j: u32;
    for i in (0..end).rev() {
        s = (s >> 8) | (block_data[i]) << 8;
        j = (freq_tab[s as usize]) - 1;
        freq_tab[s as usize] = j;
        bwt_ptr[j as usize] = i as u32;
    }

    // Initialize big_done
    let mut big_done = vec![false; 256];
    // Initialize running_order
    let mut running_order = (0..=255_u8).fold(vec![], |mut v: Vec<u8>, n| {
        v.push(n);
        v
    });

    // Why not just set h to 364????
    let mut h = 1;
    while h <= 256 {
        h = 3 * h + 1;
    }

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
    // Initialize how many have been quick sorted
    let mut num_q_sorted = 0;
    /*--
       Process big buckets, starting with the least full.
       Basically this is a 3-step process in which we call
       mainQSort3 to sort the small buckets [ss, j], but
       also make a big effort to avoid the calls if we can.
    --*/
    for (i, &ss) in running_order.iter().enumerate() {
        // // three lines of debug code
        // if i == 222 {
        //     error!("123: Pause here to check {}", watch_me);
        //     //println!("{:?}", bwt_ptr);
        // } // debug end
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
        const BZ_N_RADIX: i32 = 2;

        for j in 0..=255 {
            if j != ss {
                let sb = ((ss as u32) << 8) + j as u32;
                if 0 == (freq_tab[sb as usize] & SETMASK) {
                    let lo = (freq_tab[sb as usize] & CLEARMASK) as i32;
                    let hi = (freq_tab[sb as usize + 1] & CLEARMASK) as i32 - 1;
                    if hi > lo {
                        info!(
                            "   qsort [0x{:0x}, 0x{:0x}]   done {}   this {}",
                            ss,
                            j,
                            num_q_sorted,
                            hi - lo + 1
                        );
                        // one line of debug code
                        //let w1 = bwt_ptr[watch_me];
                        main_q_sort3(
                            &mut bwt_ptr,
                            &block_data,
                            &mut quadrant,
                            end,
                            lo,
                            hi,
                            BZ_N_RADIX,
                            &mut budget,
                        );
                        // if bwt_ptr[watch_me] != w1 {
                        //     error!(
                        //         "165: Pause here to check {}. Was {}, now {}.",
                        //         watch_me, w1, bwt_ptr[watch_me]
                        //     )
                        // } // debug end
                        num_q_sorted += hi - lo + 1;
                        if budget < 0 {
                            let mut bwt_data: Vec<u8> = vec![0; end];
                            let mut key = 0;
                            for i in 0..end as usize {
                                if bwt_ptr[i] == 0 {
                                    bwt_data[i] = block_data8[end - 1] as u8;
                                    key = i;
                                } else {
                                    bwt_data[i] = block_data8[bwt_ptr[i] as usize - 1] as u8
                                }
                            }
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
        let mut copy_start = vec![0_i32; 256];
        let mut copy_end = vec![0_i32; 256];
        for i in 0..256 {
            copy_start[i] = (freq_tab[(i << 8) + ss as usize] & CLEARMASK) as i32;
            copy_end[i] = (freq_tab[(i << 8) + ss as usize + 1] & CLEARMASK) as i32 - 1;
        }

        {
            let mut j = (freq_tab[(ss as usize) << 8] & CLEARMASK) as i32;
            loop {
                if j >= copy_start[ss as usize] {
                    break;
                }
                let mut k = bwt_ptr[j as usize] as i32 - 1;
                if k < 0 {
                    k += end as i32;
                };
                let c1 = block_data[k as usize];
                // // three lines of debug code
                // if c1 == watch_me as u16 {
                //     error!("212: Pause here to check {}", watch_me)
                // } // debug end
                if !big_done[c1 as usize] {
                    bwt_ptr[copy_start[c1 as usize] as usize] = k as u32;
                    copy_start[c1 as usize] += 1;
                }

                // debug code
                // if ss == 119 {
                //     warn!(
                //         "j:{}, k:{}, cs[c1]:{}, cs[ss]{}, ce[c1]{}, bwt_ptr[j]:{}.",
                //         j,
                //         k,
                //         copy_start[c1 as usize],
                //         copy_start[ss as usize],
                //         copy_end[c1 as usize],
                //         bwt_ptr[j as usize]
                //     );
                // }
                j += 1;
            }
            let mut j = ((freq_tab[(ss as usize + 1) << 8] & CLEARMASK) as i32) - 1;
            while j > copy_end[ss as usize] {
                let mut k = bwt_ptr[j as usize] as i32 - 1;
                if k < 0 {
                    k += end as i32
                }
                let c1 = block_data[k as usize];
                // // three lines of debug code
                // if c1 == watch_me as u16 {
                //     error!("242: Pause here to check {}", watch_me)
                // } // debug end

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
        if !(copy_start[ss as usize] - 1 == copy_end[ss as usize])
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
    // println!("ptr");
    // println!("{:?}", bwt_ptr);
    // println!("Quadrant");
    // println!("{:?}", quadrant);
    return (budget, key, bwt_data);
}

fn big_freq(freq_tab: &[u32], n: u32) -> u32 {
    (freq_tab[((n + 1) as usize) << 8] as u32) - (freq_tab[(n as usize) << 8] as u32)
}
