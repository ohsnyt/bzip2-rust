use super::freq_count::freqs;
use crate::compression::compress::Block;
use log::error;

const RUNA: u16 = 0;
const RUNB: u16 = 1;
const ZERO_BOMB: usize = 2 * 1024 * 1024;

/// Does Move-To-Front transforma and Run-Length-Encoding 2 prior to the huffman stage.
/// Gets BWT output from block.data.
/// Puts transformed u16 data into block.rle2 and frequency count into block.freqs.
pub fn rle2_mtf_encode(block: &mut Block) {
    // DEBUGGING
    // let mut file = std::fs::OpenOptions::new()
    //     .write(true)
    //     .create(true)
    //     .append(false)
    //     .open(&"test.data")
    //     .expect("failed to open");
    // std::io::Write::write_all(&mut file, &block.data);

    //println!("Test is good: {:?}", test(&block.data));

    // Create a custom index of the input, using an array for speed
    // Start by finding every u8 in the input.
    let mut bool_array = vec![false; 256];
    for i in &block.data {
        bool_array[*i as usize] = true;
    }
    let (mut mtf_index, _) = bool_array.iter().enumerate().fold(
        ([0_u8; 256], 0_usize),
        |(mut array, mut i), (s, &b)| {
            if b {
                array[i] = s as u8;
                i += 1
            };
            (array, i)
        },
    );

    // Get the EOB value from the bool_array
    block.eob = bool_array.iter().filter(|el| **el).count() as u16 + 1;

    // // For speed in doing the move-to-front, build an array of valid symbols in use
    // let mut entries = Vec::with_capacity(block.eob as usize);
    // bool_array.iter().enumerate().for_each(|(i, &tf)| {
    //     if tf == true {
    //         entries.push(i)
    //     }
    // });

    // Create the symbol map from the bool_array
    block.sym_map = encode_sym_map_from_bool_map(&bool_array);

    // We are now done with the bool array
    drop(bool_array);

    // With the index, we can do the MTF (and RLE2)
    // Initialize a zero counter
    let mut zeros = 0_usize;
    // Initialize an index into the output vec (block.rle2)
    let mut out_idx = 0_usize;
    // Size the rle2
    block.rle2 = vec![0_u16; block.data.len() + 1];
    // Set an initial max value for updating the mtf index
    //let mut last_index = 0_usize;

    // ...then do the transform (VecDeque saves a tiny bit of time over a vec)
    for byte in &block.data {
        let mut idx = mtf_index.iter().position(|c| c == byte).unwrap();
        if idx == 0 {
            zeros += 1;
            continue;
        }
        // Not a zero, so output any pending zeros first
        match zeros {
            0 => {} // Do nothing.
            1 => {
                block.rle2[out_idx] = 0;
                block.freqs[0] += 1;
                out_idx += 1;
                // Reset zeros counter
                zeros = 0;
            }
            2 => {
                block.rle2[out_idx] = 1;
                block.freqs[1] += 1;
                out_idx += 1;
                // Reset zeros counter
                zeros = 0;
            }
            mut n => {
                n -= 1;
                loop {
                    // Output the appropriate RUNA/RUNB
                    block.rle2[out_idx] = (n & 1) as u16;
                    // Update the appropriate RUNA/RUNB frequency count
                    block.freqs[n & 1] += 1;
                    // Update the output index
                    out_idx += 1;
                    // adjust the zeros counter
                    if n < 2 {
                        break;
                    }
                    n = (n - 2) >> 1;
                }
                // Reset zeros counter
                zeros = 0;
            }
        }
        // Update the frequency count
        block.freqs[idx] += 1;
        // Then output the data
        block.rle2[out_idx] = idx as u16 + 1;
        out_idx += 1;

        // DEBUGGING TEST
        //if out_idx as usize > 900019 {
        //    println!("{}", out_idx);
        //}

        // Shift each index in front of the current byte index. Do this first in blocks for speed.
        let temp_sym = mtf_index[idx as usize];

        while idx > 7 {
            mtf_index[idx as usize] = mtf_index[idx as usize - 1];
            mtf_index[idx as usize - 1] = mtf_index[idx as usize - 2];
            mtf_index[idx as usize - 2] = mtf_index[idx as usize - 3];
            mtf_index[idx as usize - 3] = mtf_index[idx as usize - 4];
            mtf_index[idx as usize - 4] = mtf_index[idx as usize - 5];
            mtf_index[idx as usize - 5] = mtf_index[idx as usize - 6];
            mtf_index[idx as usize - 6] = mtf_index[idx as usize - 7];
            mtf_index[idx as usize - 7] = mtf_index[idx as usize - 8];
            idx -= 8;
        }
        while idx > 3 {
            mtf_index[idx as usize] = mtf_index[idx as usize - 1];
            mtf_index[idx as usize - 1] = mtf_index[idx as usize - 2];
            mtf_index[idx as usize - 2] = mtf_index[idx as usize - 3];
            mtf_index[idx as usize - 3] = mtf_index[idx as usize - 4];
            idx -= 4;
        }
        // ...then clean up any odd ones
        while idx > 0 {
            mtf_index[idx as usize] = mtf_index[idx as usize - 1];
            idx -= 1;
        }
        // ...and finally put the "new" symbol index at the front of the index.
        mtf_index[0] = temp_sym;
    }

    // Write any trailing zeros
    match zeros {
        0 => {} // Do nothing.
        1 => {
            block.rle2[out_idx] = 0;
            block.freqs[0] += 1;
            out_idx += 1;
        }
        2 => {
            block.rle2[out_idx] = 1;
            block.freqs[1] += 1;
            out_idx += 1;
        }
        mut n => {
            n -= 1;
            loop {
                // Output the appropriate RUNA/RUNB
                block.rle2[out_idx] = (n & 1) as u16;
                // Update the appropriate RUNA/RUNB frequency count
                block.freqs[n & 1] += 1;
                // Update the output index
                out_idx += 1;
                // adjust the zeros counter
                if n < 2 {
                    break;
                }
                n = (n - 2) >> 1;
            }
        }
    }
    // Add the EOB symbol to the end
    block.rle2[out_idx] = block.eob;
    out_idx += 1;

    // DEBUGGING TEST
    // if out_idx as usize > 900019 {
    //     println!("{}", out_idx);
    // }

    // Truncate the vec to the actual data.
    block.rle2.truncate(out_idx);
    block.end = out_idx as u32;
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

    //Create the freq vec
    let freq = freqs(&out);

    (out, freq)
}

const BIT_MASK: u16 = 0x8000;

/// Takes an array of all u8s used at the BWT stage and returns a
/// bzip2 symbol map. Assumes at least one symbol exists.
fn encode_sym_map_from_bool_map(symbols: &[bool]) -> Vec<u16> {
    /*
       There are 256 possible u8s, which equals 16 sets of 16 u8s. This means we can indicate
       the existence of every u8 in the input by setting bits in 16 16-bit words.
       Since many files contain only a subset of the full u8 set, we can save space by only
       including those 16-bit words have have at least one bit set. Bzip2 prefaces the set of 16
       words with another 16 bit word which has a bit set for each word that is included following.
    */
    let mut sym_maps: Vec<u16> = vec![0; 17]; // Index and 16 maps

    // We use bit masks to set the bit indicating which map and which symbol in that map is used
    // The first 8 bits indicate which map, and the second 8 bits indicate which symbol
    // Eg 'A' is 0100_0001, so map 8 (0100) would have bit 1 (0001) set.
    symbols.iter().enumerate().for_each(|(idx, &sym)| {
        // Whenever we find a symbol, set the map index and the symbol bit
        if sym {
            // Idx/16 (idx>>4) marks the map index
            sym_maps[0] |= BIT_MASK >> (idx >> 4);
            // The last 15 bits if idx marks the symbol within that set
            sym_maps[1 + (idx >> 4)] |= BIT_MASK >> (idx & 15)
        }
    });
    for map in &sym_maps {
        log::trace!("\r\x1b[43m{:0>16b}     \x1b[0m", map);
    }

    // Return only those vecs that have bits set.
    sym_maps.retain(|&map| map > 0);
    sym_maps
}

// fn test(data: &[u8]) -> bool {
//     let mut test_data: Vec<u8> = vec![0_u8;70000];
//     let mut file = std::fs::File::open("test.data").unwrap();
//     test_data.truncate(data.len());
//     std::io::Read::read(&mut file, &mut test_data).expect("Couldn't read test data");

//     for i in 0..data.len() {
//         if data[i] != test_data[i] {
//             println!("Mismatch at {}: '{}|{}'", i, data[i], test_data[i]);
//         }
//     }
//     test_data == data
// }
