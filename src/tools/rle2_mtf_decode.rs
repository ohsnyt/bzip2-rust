use log::error;
use rayon::prelude::*;

const RUNA: u16 = 0;
const RUNB: u16 = 1;
const ZERO_BOMB: usize = 2 * 1024 * 1024;

/* /// Does run-length-decoding from rle2_encode.
pub fn rle2_mtf_decode(data_in: &[u16], mut mtf_index: &mut Vec<u8>, size: usize) -> Vec<u8> {
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
    let halfway = end << 2;

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
                zeros += bit_multiplier << 1;
                bit_multiplier <<= 1;
            }

            // Found a "normal" symbol
            n => {
                // Output zer data, if any
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

                // Then output the symbol (location is one less than n)
                let loc = n as usize - 1;
                out[index] = mtf_index[loc];

                // Increment the index
                index += 1;

                // And adjust the mtf_index for the next symbol.
                /* NOTE:
                Straight remove/insert is SLIGHTLY faster than straight swap.
                Mixing in any ratio is slow than either alone.
                */
                if loc < halfway {
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

    // Truncate the vec to the actual data, removing the eob marker.
    out.truncate(index - 1);
    out
}
 */
/// Does run-length-decoding from rle2_encode.
pub fn rle2_mtf_decode_fast(
    data_in: &[u16],
    mtf_index: &mut Vec<u8>,
    size: usize,
) -> (Vec<u32>, Vec<u32>) {
    // Initialize output buffer
    let mut out = vec![0_u32; size];

    // Initialize counters
    let mut zeros = 0_usize;
    let mut bit_multiplier = 1;
    let mut index = 0_usize;

    // Add (bogus) eob symbol to the mtf_index (symbol set)
    mtf_index.push(0);

    // For speed bump in unsafe code below
    //let end = mtf_index.len();
    //let halfway = end << 2;

    // iterate through the input, doing the conversion as we find RUNA/RUNB sequences
    for &symbol in data_in {
        match symbol {
            // If we found RUNA, do magic to calculate how many zeros we need
            RUNA => {
                zeros += bit_multiplier;
                bit_multiplier <<= 1;
            }
            // If we found RUNB, do magic to calculate how many zeros we need
            RUNB => {
                zeros += bit_multiplier << 1;
                bit_multiplier <<= 1;
            }

            // Found a "normal" symbol
            n => {
                // Output zero data, if any
                if zeros > 0 {
                    zero_bomb(zeros);
                    for symbol in out.iter_mut().skip(index).take(zeros + 1) {
                        *symbol = mtf_index[0] as u32;
                    }

                    // Adjust the counters
                    index += zeros;
                    bit_multiplier = 1;
                    zeros = 0;
                }

                // Then output the symbol (location is one less than n)
                let mut loc = n as usize - 1;
                out[index] = mtf_index[loc] as u32;

                // Increment the index
                index += 1;

                // If the index is less than 16 elements into the vec,
                if loc < 16 {
                    // Shift each index at the front of mtfa "forward" one. Do this first in blocks of 4 for speed.
                    let temp_sym = mtf_index[loc];

                    while loc > 3 {
                        mtf_index[loc] = mtf_index[loc - 1];
                        mtf_index[loc - 1] = mtf_index[loc - 2];
                        mtf_index[loc - 2] = mtf_index[loc - 3];
                        mtf_index[loc - 3] = mtf_index[loc - 4];
                        loc -= 4;
                    }
                    // ...then clean up any odd ones
                    while loc > 0 {
                        mtf_index[loc] = mtf_index[loc - 1];
                        loc -= 1;
                    }
                    // ...and finally put the "new" symbol index at the front of the index.
                    mtf_index[0] = temp_sym;
                } else {
                    /* general case */
                    let sym = mtf_index.remove(loc);
                    mtf_index.insert(0, sym as u8)
                }
            }
        }
    }

    // Truncate the vec to the actual data, removing the eob marker.
    out.truncate(index - 1);

    //Create the freq vec using a parallel approach
    let freq = out
        .par_iter()
        .fold(
            || vec![0_u32; 256],
            |mut freqs, &el| {
                freqs[el as usize] += 1;
                freqs
            },
        )
        .reduce(
            || vec![0_u32; 256],
            |s, f| s.iter().zip(&f).map(|(a, b)| a + b).collect::<Vec<u32>>(),
        );

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
