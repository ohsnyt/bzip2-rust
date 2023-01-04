use log::error;
use rayon::prelude::*;

use crate::compression::compress::Block;

const RUNA: u16 = 0;
const RUNB: u16 = 1;
const ZERO_BOMB: usize = 2 * 1024 * 1024;

/// Does run-length-decoding from rle2_encode.
pub fn rle2_mtf_decode(data_in: &[u16], mtf_index: &mut Vec<u8>, size: usize) -> Vec<u8> {
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

/// Does Move-To-Front transforma and Run-Length-Encoding 2 prior to the huffman stage.
/// Gets BWT output from block.data.
/// Puts transformed u16 data into block.temp_vec and frequency count into block.freqs.
pub fn rle2_mtf_encode(block: &mut Block) {
    // Create a custom index of the input, using an array for speed
    // Start by finding every u8 in the input.
    let mut bool_array = vec![false; 256];
    for i in &block.data {
        bool_array[*i as usize] = true;
    }
    // Then initialize an index array of Option<usize> to None.
    let mut mtf_index: [Option<usize>; 256] = [None; 256];
    // For every u8 in the input, add an appropriate index number from 0..
    let mut idx = 0_usize;
    bool_array.iter().enumerate().for_each(|(i, &b)| {
        if b {
            mtf_index[i] = Some(idx);
            idx += 1
        }
    });

    // Get the EOB value from the bool_array
    block.eob = bool_array.iter().filter(|el| **el).count() as u16;

    // We are now done with the bool array
    drop(bool_array);

    // With the index, we can do the MTF (and RLE2)
    // Initialize a zero counter
    let mut zeros = 0_usize;
    // Initialize an index into the output vec (block.temp_vec)
    let mut idx = 0_usize;
    // Size the temp_vec
    block.temp_vec = vec![0_u16; block.data.len()];

    // iterate through the input, doing the conversion as we create runs of zeros
    for &byte in block.data.iter() {
        // Get the byte index number stored in the MTF index
        match mtf_index[byte as usize] {
            None => {} // Should never happen
            // If it is a zero, count it
            Some(0) => zeros += 1,
            // Anything else, output the zero sequence and then the next index
            Some(index) => {
                match zeros {
                    0 => {} // Do nothing.
                    1 => {
                        block.temp_vec[idx] = 0;
                        block.freqs[0] += 1;
                        idx += 1;
                        zeros = 0;
                    }
                    2 => {
                        block.temp_vec[idx] = 1;
                        block.freqs[1] += 1;
                        idx += 1;
                        zeros = 0;
                    }
                    mut n => {
                        n -= 1;
                        loop {
                            // Output the appropriate RUNA/RUNB
                            block.temp_vec[idx] = (n & 1) as u16;
                            // Update the appropriate RUNA/RUNB frequency count
                            block.freqs[n & 1] += 1;
                            // Update the output index
                            idx += 1;
                            // adjust the zeros counter
                            if n < 2 {
                                break;
                            }
                            n = (n - 2) >> 1;
                        }
                        zeros = 0;
                    }
                }
                // Output next index (incremented by 1 for the RUNB element)
                block.temp_vec[idx] = index as u16 + 1;
                idx += 1;
                // Update the appropriate frequency count
                block.freqs[index + 1] += 1;

                // Shift the indecies
                for i in 0..mtf_index.len() {
                    if mtf_index[i].is_some() {
                        if mtf_index[i] > Some(index) {
                            // Do nothing
                        } else if mtf_index[i] == Some(index) {
                            mtf_index[i] = Some(0)
                        } else {
                            mtf_index[i] = Some(mtf_index[i].unwrap() + 1);
                        }
                    }
                }
            }
        }
    }
    // Writing any trailing zeros
    match zeros {
        0 => {} // Do nothing.
        1 => {
            block.temp_vec[idx] = 0;
            block.freqs[0] += 1;
            idx += 1;
        }
        2 => {
            block.temp_vec[idx] = 1;
            block.freqs[1] += 1;
            idx += 1;
        }
        mut n => {
            n -= 1;
            loop {
                // Output the appropriate RUNA/RUNB
                block.temp_vec[idx] = (n & 1) as u16;
                // Update the appropriate RUNA/RUNB frequency count
                block.freqs[n & 1] += 1;
                // Update the output index
                idx += 1;
                // adjust the zeros counter
                if n < 2 {
                    break;
                }
                n = (n - 2) >> 1;
            }
        }
    }
    // Add the EOB symbol to the end
    block.temp_vec[idx] = block.eob;
    idx += 1;

    // Truncate the vec to the actual data.
    block.temp_vec.truncate(idx);
}

/// Watch for malicious input
fn zero_bomb(zeros: usize) {
    // Blow up if the run is too big - this should be more elegant in the future
    if zeros > ZERO_BOMB {
        error!("Run of zeros exceeded a million - probably input bomb.");
        std::process::exit(100)
    }
}

/// Does run-length-decoding and MTF decoding.
/// Takes huffman decoder data, symbol set, and max block size.
/// Returns ascii data for bwt transform, plus frequency count of the data.
pub fn rle2_mtf_decode_fast(
    data_in: &[u16],
    mtf_index: &mut Vec<u8>,
    size: usize,
) -> (Vec<u8>, Vec<u32>) {
    // Initialize output buffer
    let mut out = vec![0_u8; size];

    // Initialize counters
    let mut zeros = 0_usize;
    let mut bit_multiplier = 1;
    let mut index = 0_usize;

    // iterate through the input (less EOB), doing the conversion as we find RUNA/RUNB sequences
    for &rle2_code in data_in.iter().take(data_in.len() - 1) {
        match rle2_code {
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

            // Found a "normal" rle2_code
            n => {
                // Output zeros from RUNA/RUNB sequences, if any
                if zeros > 0 {
                    zero_bomb(zeros);
                    for repeat in out.iter_mut().skip(index).take(zeros) {
                        *repeat = mtf_index[0];
                    }

                    // Adjust the RUNA/RUNB sequence counters
                    index += zeros;
                    bit_multiplier = 1;
                    zeros = 0;
                }

                // Convert the RLE2_code into an MTF_code
                let mut mtf_code = n as usize - 1;
                // And output an byte from the MTF index
                out[index] = mtf_index[mtf_code];

                // Increment the index
                index += 1;

                // If this index is less than 16 elements into the vec,
                if mtf_code < 16 {
                    // Shift each index at the front of mtfa "forward" one. Do this first in blocks of 4 for speed.
                    let temp_sym = mtf_index[mtf_code];

                    while mtf_code > 3 {
                        mtf_index[mtf_code] = mtf_index[mtf_code - 1];
                        mtf_index[mtf_code - 1] = mtf_index[mtf_code - 2];
                        mtf_index[mtf_code - 2] = mtf_index[mtf_code - 3];
                        mtf_index[mtf_code - 3] = mtf_index[mtf_code - 4];
                        mtf_code -= 4;
                    }
                    // ...then clean up any odd ones
                    while mtf_code > 0 {
                        mtf_index[mtf_code] = mtf_index[mtf_code - 1];
                        mtf_code -= 1;
                    }
                    // ...and finally move this index to the front.
                    mtf_index[0] = temp_sym;
                } else {
                    /* general case */
                    let sym = mtf_index.remove(mtf_code);
                    mtf_index.insert(0, sym as u8)
                }
            }
        }
    }
    // Output trailing zeros from RUNA/RUNB sequences, if any
    if zeros > 0 {
        zero_bomb(zeros);
        for repeat in out.iter_mut().skip(index).take(zeros) {
            *repeat = mtf_index[0];
        }

        // Adjust the index as needed
        index += zeros;
    }

    // Truncate the vec to the actual data.
    // (Index is incremented after writing the symbol, so must be decremented by one here.)
    out.truncate(index);

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
