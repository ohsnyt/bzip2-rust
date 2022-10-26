use std::collections::VecDeque;

use log::error;

const RUNA: u16 = 0;
const RUNB: u16 = 1;
const ZERO_BOMB: usize = 2 * 1024 * 1024;

/// Does run-length-decoding from rle2_encode.
pub fn rle2_mtf_decode(data_in: &[u16], mut mtf_index: &mut Vec<u8>, size: usize) -> Vec<u8> {
    // let time = std::time::Instant::now();
    // let mut time_mtf = std::time::Duration::new(0, 0);
    // let mut time_zeros = Duration::new(0, 0);
    // let mut time_out = Duration::new(0, 0);

    // Initialize output buffer
    let mut out = vec![0; size];

    // Initialize counters
    let mut zeros = 0_usize;
    let mut bit_multiplier = 1;
    let mut index = 0_usize;

    // Add (bogus) eob symbol to the mtf_index (symbol set)
    mtf_index.push(0);

    // For speed bump in unsafe code below
    let end = mtf_index.len();

    // let time_init = time.elapsed();
    // iterate through the input, doing the conversion as we find RUNA/RUNB sequences
    for &symbol in data_in {
        match symbol {
            // If we found RUNA, do magic to calculate how many zeros we need
            RUNA => {
                // let timer = Instant::now();
                zeros += bit_multiplier;
                bit_multiplier <<= 1;
                // time_zeros += timer.elapsed();
            }
            // If we found RUNB, do magic to calculate how many zeros we need
            RUNB => {
                // let timer = Instant::now();
                zeros += (bit_multiplier << 1);
                bit_multiplier <<= 1;
                // time_zeros += timer.elapsed();
            }
            // If we found symbol 2...
            // Do a swap when there are only two items provides a very minimal preformance increase
            // 2 => {
            //     // First output any pending run of zeros as mtf[0].
            //     if zeros > 0 {
            //         zero_bomb(zeros);
            //         for symbol in out.iter_mut().skip(index).take(zeros + 1) {
            //             *symbol = mtf_index[0]
            //         }
            //         // Adjust the counters
            //         index += zeros;
            //         bit_multiplier = 1;
            //         zeros = 0;
            //     }
            //     // Then output the symbol (one less than n) and increment the index
            //     out[index] = mtf_index[1];
            //     index += 1;

            //     // Do the swap of the first two items in the mtf_index
            //     mtf_index.swap(0 as usize, 1);
            // }
            // // Swapping more than just 2 slows the performance.
            // // 3 => ...

            // Anything other symbol index, first output any pending run of zeros as mtf[0].
            n => {
                // let timer = Instant::now();
                if zeros > 0 {
                    zero_bomb(zeros);
                    for symbol in out.iter_mut().skip(index).take(zeros + 1) {
                        *symbol = mtf_index[0]
                    }
                    // Adjust the counters
                    index += zeros;
                    bit_multiplier = 1;
                    zeros = 0;
                }
                // time_zeros += timer.elapsed();

                // let timer = Instant::now();
                // Then output the symbol (location is one less than n)
                let loc = n as usize - 1;
                out[index] = mtf_index[loc];

                // Increment the index
                index += 1;

                // time_out += timer.elapsed();

                // And adjust the mtf_index for the next symbol.
                /* NOTE:
                Straight remove/insert is SLIGHTLY faster than straight swap.
                Mixing in any ratio is slow than either alone.
                */
                // let timer = std::time::Instant::now();
                if loc < end << 2 {
                    let y = mtf_index[loc..=loc].as_mut_ptr() as *mut [u8; 1];
                    for i in 0..loc as usize {
                        let x = mtf_index[i..=i].as_mut_ptr() as *mut [u8; 1];
                        unsafe { std::ptr::swap(x, y) };
                    }
                } else {
                    let sym = mtf_index.remove(loc);
                    mtf_index.insert(0, sym as u8)
                }
                // time_mtf += timer.elapsed();
            }
        }
    }
    // let timer = Instant::now();
    // Truncate the vec to the actual data, removing the eob marker.
    out.truncate(index - 1);
    // let time_trunc = timer.elapsed();

    // println!("     Init time:{:?}", time_init);
    // println!("MTF index time:{:?}", time_mtf);
    // println!("Zero calc time:{:?}", time_zeros);
    // println!("   Output time:{:?}", time_out);
    // println!(" Truncate time:{:?}", time_trunc);
    // println!("Total MTF time:{:?}", time.elapsed());
    // println!("??  Match time:{:?}", time.elapsed()- time_init - time_mtf - time_zeros - time_out);
    out
}

/// Does run-length-decoding from rle2_encode.
pub fn rle2_mtf_decode_fast(data_in: &[u16], mut mtf_index: &mut Vec<u8>, size: usize) -> (Vec<u32> , [u32; 256]) {
    // Initialize output buffer
    let mut out = vec![0_u32; size];
    // Initialize frequency counter as an array (for speed)
    let mut freq = [0_u32; 256];

    // Initialize counters
    let mut zeros = 0_usize;
    let mut bit_multiplier = 1;
    let mut index = 0_usize;

    // Add (bogus) eob symbol to the mtf_index (symbol set)
    mtf_index.push(0);

    // For speed bump in unsafe code below
    let end = mtf_index.len();

    // let time_init = time.elapsed();
    // iterate through the input, doing the conversion as we find RUNA/RUNB sequences
    for &symbol in data_in {
        match symbol {
            // If we found RUNA, do magic to calculate how many zeros we need
            RUNA => {
                // let timer = Instant::now();
                zeros += bit_multiplier;
                bit_multiplier <<= 1;
                // time_zeros += timer.elapsed();
            }
            // If we found RUNB, do magic to calculate how many zeros we need
            RUNB => {
                // let timer = Instant::now();
                zeros += (bit_multiplier << 1);
                bit_multiplier <<= 1;
                // time_zeros += timer.elapsed();
            }
            // Found a "normal" symbol
            n => {
                // Output zero data, if any
                if zeros > 0 {
                    zero_bomb(zeros);
                    for symbol in out.iter_mut().skip(index).take(zeros + 1) {
                        *symbol = mtf_index[0] as u32;
                    }
                    // Update the frequency counter
                    freq[mtf_index[0] as usize] += zeros as u32;

                    // Adjust the counters
                    index += zeros;
                    bit_multiplier = 1;
                    zeros = 0;
                }

                // Then output the symbol (location is one less than n)
                let mut loc = n as usize - 1;
                out[index] = mtf_index[loc] as u32;

                // Update the frequency counter
                freq[mtf_index[loc] as usize] += 1;

                // Increment the index
                index += 1;

                // And adjust the mtf_index for the next symbol.

                // If the index is 0..15,
                if (loc < 16) {
                    /* avoid general-case expense */
                    // Shift each index at the front of mtfa "forward" one. Do this first in blocks of 4
                    let temp_sym = mtf_index[loc];

                    while (loc > 3) {
                        mtf_index[loc] = mtf_index[loc - 1];
                        mtf_index[loc - 1] = mtf_index[loc - 2];
                        mtf_index[loc - 2] = mtf_index[loc - 3];
                        mtf_index[loc - 3] = mtf_index[loc - 4];
                        loc -= 4;
                    }
                    // ...then clean up any odd ones
                    while (loc > 0) {
                        mtf_index[loc] = mtf_index[loc - 1];
                        loc -= 1;
                    }
                    // ...and finally put the "new" symbol index at the front of the index.
                    mtf_index[0] = temp_sym;
                } else {
                    /* general case */
                    if loc < end << 2 {
                        let y = mtf_index[loc..=loc].as_mut_ptr() as *mut [u8; 1];
                        for i in 0..loc as usize {
                            let x = mtf_index[i..=i].as_mut_ptr() as *mut [u8; 1];
                            unsafe { std::ptr::swap(x, y) };
                        }
                    } else {
                        let sym = mtf_index.remove(loc);
                        mtf_index.insert(0, sym as u8)
                    }
                }
            }
        }
    }

    // Truncate the vec to the actual data, removing the eob marker.
    out.truncate(index - 1);
    (out, freq)
}

/// Watch for malicious input
fn zero_bomb(zeros: usize) {
    // Blow up if the run is too big - this should be more elegant in the future
    if zeros > ZERO_BOMB {
        error!("Run of zeros exceeded a million - probably input bomb.");
        std::process::exit(100)
    }
}
