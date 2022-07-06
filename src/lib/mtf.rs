use std::collections::VecDeque;

use super::{compress::Block, symbol_map::encode_sym_map};

/// Encode data using Move To Front transform. Could bring RLE2 into here.
pub fn mtf_encode(block: &mut Block) {
    // Create a custom index of the input.
    // Note: ds: This is 10 times faster than sort/dedup of a vec
    let mut v = vec![false; 256];
    for i in &block.data {
        v[*i as usize] = true;
    }
    let mut index = v
        .iter()
        .enumerate()
        .filter_map(|(s, &b)| if b { Some(s as u8) } else { None })
        .collect::<VecDeque<u8>>();

    // Create the symbol map while we have the input data
    block.sym_map = encode_sym_map(&index);

    // ...then do the transform (VecDeque saves a tiny bit of time over a vec)
    for i in 0..block.end {
        let byte = block.data[i as usize];
        let idx = index.iter_mut().position(|c| c == &byte).unwrap() as u8;
        block.data[i as usize] = idx;
        if idx != 0 {
            let _ = index.remove(idx as usize);
            index.push_front(byte);
        }
    }
}

