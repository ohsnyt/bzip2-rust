use std::collections::VecDeque;

use log::error;

const RUNA: u16 = 0;
const RUNB: u16 = 1;
const ZERO_BOMB: usize = 2 * 1024 * 1024;

/// Does run-length-decoding from rle2_encode.
pub fn rle2_mtf_decode(data_in: &[u16], out: &mut Vec<u8>, mut mtf_index: &mut Vec<u8>) {
    // Initialize counters
    let mut zeros = 0_usize;
    let mut bit_multiplier = 1;
    let mut index = 0_usize;

    // Add (bogus) eob symbol to the mtf_index (symbol set)
    mtf_index.push(0);

    // For speed bump in unsafe code below
    let end = mtf_index.len();

    // iterate through the input, doing the conversion as we find RUNA/RUNB sequences
    for symbol in data_in {
        // Blow up if the run is too big - this should be more elegant in the future
        if zeros > ZERO_BOMB {
            error!("Run of zeros exceeded a million - probably input bomb.");
            std::process::exit(100)
        }
        match *symbol {
            // If we found RUNA, do magic to calculate how many zeros we need
            RUNA => {
                zeros += bit_multiplier;
                bit_multiplier <<= 1;
            }
            // If we found RUNB, do magic to calculate how many zeros we need
            RUNB => {
                zeros += (bit_multiplier << 1);
                bit_multiplier <<= 1;
            }
            // Swapping when there are only two items provides a very minimal preformance increase
            2 => {
                if zeros > 0 {
                    for symbol in out.iter_mut().skip(index).take(zeros + 1) {
                        *symbol = mtf_index[0]
                    }
                    // Adjust the counters
                    index += zeros;
                    bit_multiplier = 1;
                    zeros = 0;
                }
                // Then output the symbol (one less than n)
                out[index] = mtf_index[1];

                // Increment the index
                index += 1;

                mtf_index.swap(0 as usize, 1);
            }
            // Adding one more slows the performance.
            // 3 => {
            //     if zeros > 0 {
            //         for symbol in out.iter_mut().skip(index).take(zeros + 1) {
            //             *symbol = mtf_index[0]
            //         }
            //         // Adjust the counters
            //         index += zeros;
            //         bit_multiplier = 1;
            //         zeros = 0;
            //     }
            //     // Then output the symbol (one less than n)
            //     out[index] = mtf_index[2];

            //     // Increment the index
            //     index += 1;

            //     mtf_index.swap(0 as usize, 2);
            //     mtf_index.swap(1 as usize, 2);
            // } // Anything else, first output any pending run of zeros as mtf[0].
            n => {
                if zeros > 0 {
                    for symbol in out.iter_mut().skip(index).take(zeros + 1) {
                        *symbol = mtf_index[0]
                    }
                    // Adjust the counters
                    index += zeros;
                    bit_multiplier = 1;
                    zeros = 0;
                }
                // Then output the symbol (one less than n)
                out[index] = mtf_index[n as usize - 1];

                // Increment the index
                index += 1;

                // And adjust the mtf_index for the next symbol.
                let sym = mtf_index.remove(n as usize - 1);
                mtf_index.insert(0, sym as u8)
            }
        }
    }
    // Truncate the vec to the actual data, removing the eob marker.
    out.truncate(index - 1);
}
