use super::symbol_map::encode_sym_map;
use crate::Block;

/// Encode data using Move To Front transform. Could bring RLE2 into here.
pub fn mtf_encode(block: &mut Block) {
    // Create a custom index of the input. Put it into an array for speed.
    let mut v = vec![false; 256];
    for i in &block.data {
        v[*i as usize] = true;
    }
let (mut index , _) = v
        .iter()
        .enumerate()
        .fold(([0_u8; 256], 0_usize), |(mut array, mut i), (s, &b)| {if b { array[i] = s as u8; i+=1}; (array, i) } );

    // Create the symbol map while we have the input data
    block.sym_map = encode_sym_map(&index);

    // ...then do the transform (VecDeque saves a tiny bit of time over a vec)
    for i in 0..block.end {
        let byte = block.data[i as usize];
        let mut idx = index.iter().position(|c| c == &byte).unwrap();
        block.data[i as usize] = idx as u8;

        // Shift each index in front of the current byte index. Do this first in blocks for speed.
        let temp_sym = index[idx as usize];

        while idx > 7 {
            index[idx as usize] = index[idx as usize - 1];
            index[idx as usize - 1] = index[idx as usize - 2];
            index[idx as usize - 2] = index[idx as usize - 3];
            index[idx as usize - 3] = index[idx as usize - 4];
            index[idx as usize - 4] = index[idx as usize - 5];
            index[idx as usize - 5] = index[idx as usize - 6];
            index[idx as usize - 6] = index[idx as usize - 7];
            index[idx as usize - 7] = index[idx as usize - 8];
            idx -= 8;
        }
        while idx > 3 {
            index[idx as usize] = index[idx as usize - 1];
            index[idx as usize - 1] = index[idx as usize - 2];
            index[idx as usize - 2] = index[idx as usize - 3];
            index[idx as usize - 3] = index[idx as usize - 4];
            idx -= 4;
        }
        // ...then clean up any odd ones
        while idx > 0 {
            index[idx as usize] = index[idx as usize - 1];
            idx -= 1;
        }
        // ...and finally put the "new" symbol index at the front of the index.
        index[0] = temp_sym;
    }
}


// /// Encodes runs of zeros as RUNA/RUNB, recording how many RUNA and RUNB were used.
// fn rle2_encode_runs(r: u32, freqs: &mut [u32]) -> Vec<u16> {
//     // if the run is 0, return empty vec
//     if r == 0 {
//         return vec![];
//     }
//     // otherwise, reduce the run count by 1
//     let mut run = r - 1;
//     // prepare the return vec
//     let mut out: Vec<u16> = vec![];
//     // while the last bit > 1, push the last bit, increment the count, decrement run
//     loop {
//         let bit = (run & 1) as u16;
//         out.push(bit);
//         freqs[bit as usize] += 1;
//         if run < 2 {
//             break;
//         }
//         run = (run - 2) >> 1; // >> 1 is faster than /2
//     }
//     // and return the unique bzip2 run of RUNA/RUNB (bit 0, bit 1)
//     out
// }

