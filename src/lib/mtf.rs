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

/// Decode data using Move To Front transform. Could bring RLE2 into here.
/// Decoding requires a sorted symbol map index.
pub fn mtf_decode(raw: &[u8], index: Vec<u8>) -> Vec<u8> {
    raw.iter()
        .enumerate()
        .fold(
            (vec![0; raw.len()], index),
            |(mut mtf_v, mut s), (idx, &x)| {
                mtf_v[idx] = s[x as usize];
                let tmp = s.remove(x as usize);
                s.insert(0, tmp);
                (mtf_v, s)
            },
        )
        .0
        .into_iter()
        .map(|c| c as u8)
        .collect::<Vec<u8>>()
}
