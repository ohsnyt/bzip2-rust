use std::collections::VecDeque;

const BIT_MASK: u16 = 0x8000;

/// Similar to makeMaps_e
/// Takes a sorted deduped vec of all symbols used in the input and
/// creates the unique bzip2 symbol map of symbols encoded as u16s
pub fn encode_sym_map(symbols: &VecDeque<u8>) -> Vec<u16> {
    let mut symbol_maps: Vec<u16> = vec![0; 17];

    for byte in symbols.iter() {
        let l1 = byte >> 4; // Divide by 16 (>>4) to find which map set the bit mask belongs to
        symbol_maps[0] |= BIT_MASK >> l1; // Mask the appropriate set
        symbol_maps[1 + l1 as usize] |= BIT_MASK >> (byte & 15); // (&15 = %16) index to the map, and set the mask there
    }
    // Get ready to return only those vecs that have bits set.
    for idx in (0..17).rev() {
        if symbol_maps[idx] == 0 {
            symbol_maps.remove(idx);
        }
    }
    symbol_maps
}

/// Takes the unique bzip2 symbol map and returns a sorted vec of all
/// u8s used in the input.
pub fn decode_sym_map(symbol_map: &[u16]) -> Vec<u8> {
    /*
    Symbol_index is a map of the presense/absense of blocks of u8s in the input data.
    For example, if the first bit of maps[0] is a zero, then none of the u8s from 0-15 were
    present in the input file, AND there would be no u16 needed to mark any of those.
    If the second bit of maps[0] is a one, then at least one u8 from the range of 16-23 was present
    in the input. That means the next u16 would be a bit map for this block of u8s with 1s and 0s
    indicating the presence / absense of those u8s. Etc.
    */
    //
    let mut result: Vec<u8> = Vec::with_capacity(256);
    // Set a counter for the number of maps
    let mut map_idx = 0;

    for block in 0..16 {
        // Check the index to see if the next bit has a block of bytes
        if (symbol_map[0] & (BIT_MASK >> block)) > 0 {
            // If so, iterate through the next u16 to find which bytes were present
            for byte_idx in 0..16_u8 {
                if (symbol_map[map_idx + 1] & (BIT_MASK >> byte_idx)) > 0 {
                    // Store this symbol on the vec. (block * 16 + byte_idx = u8 value we found)
                    result.push((block << 4) + byte_idx);
                };
            }
            map_idx += 1;
        }
    }
    result
}

#[test]
fn encode_symbol_map_test() {
    let mut x = "Making a silly test.".as_bytes().to_vec();
    x.sort_unstable();
    x.dedup();
    let x = VecDeque::from(x);
    let idx = vec![11008, 32770, 4, 17754, 6208];
    assert_eq!(idx, encode_sym_map(&x))
}
#[test]
fn encode_symbol_map_full_test() {
    let mut x: Vec<u8> = vec![];
    for i in 0..=255 {
        x.push(i)
    }
    let x = VecDeque::from(x);
    let idx = vec![0xffff; 17];
    assert_eq!(idx, encode_sym_map(&x))
}
#[test]
fn decode_symbol_map_test() {
    let maps = vec![11008, 32770, 4, 17754, 6208];
    let mut compare = "Making a silly test.".as_bytes().to_vec();
    compare.sort_unstable();
    compare.dedup();
    assert_eq!(compare, decode_sym_map(&maps));
}

#[test]
fn decode_symbol_map_full_test() {
    let maps = vec![0xffff; 17];
    let mut compare: Vec<u8> = vec![];
    for i in 0..=255 {
        compare.push(i)
    }
    compare.sort_unstable();
    compare.dedup();
    assert_eq!(compare, decode_sym_map(&maps));
}

#[test]
fn roundtrip_symbol_map_test() {
    let mut x = "Decode this.".as_bytes().to_vec();
    x.sort_unstable();
    x.dedup();
    let y = VecDeque::from(x.clone());
    let rt = encode_sym_map(&y);
    assert_eq!(decode_sym_map(&rt), x)
}
