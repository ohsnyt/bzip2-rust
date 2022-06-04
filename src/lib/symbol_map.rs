use std::collections::VecDeque;

/// Similar to makeMaps_e
/// Takes a sorted deduped vec of all symbols used in the input and
/// creates the unique bzip2 symbol map of symbols encoded as u16s
pub fn encode_sym_map(symbols: &VecDeque<u8>) -> Vec<u16> {
    let mut map_l1: u16 = 0;
    let mut map_l2: Vec<u16> = vec![0; 16];

    for byte in symbols.iter() {
        let l1 = byte / 16; // find out which map set the bit mask belongs to
        map_l1 |= 0x8000 >> l1; //mask the appropriate set
        map_l2[l1 as usize] |= 0x8000 >> (byte % 16); //index to the set and set the mask there
    }
    // Get ready to return only those vecs that have bits set.
    let mut sym_vec = Vec::with_capacity(9);
    sym_vec.push(map_l1);
    for map in map_l2 {
        if map > 0 {
            sym_vec.push(map)
        }
    }
    sym_vec.shrink_to_fit();
    sym_vec
}

/// Takes the unique bzip2 symbol map and returns a sorted vec of all
/// u8s used in the input.
pub fn decode_sym_map(maps: &[u16]) -> Vec<u8> {
    /*
    maps[0] is a map of the presense/absense of blocks of u8s in the input data.
    For example, if the first bit of maps[0] is a zero, then none of the u8s from 0-15 were
    present in the input file, AND there would be no u16 needed to mark any of those.
    If the second bit of maps[0] is a one, then at least one u8 from the range of 16-23 was present
    in the input. That means the next u16 would be a bit map for this block of u8s with 1s and 0s 
    indicating the presence / absense of those u8s. Etc.
    */
    // Get the block map.
    let map_l1: u16 = maps[0];
    // 
    let mut result: Vec<u8> = Vec::with_capacity(map_l1.count_ones() as usize);
    // offset lets us get the correct u16 in the maps vec.
    let mut offset = 1;
    for mult in 0..16_u8 {
        // Check if any u8 in this block of u8s was present in the input 
        if map_l1 >> (15 - mult) & 0x0001 == 1 {
            // If so, iterate through the next u16 to find which u8s were present, and push them on the vec.
            for idx in 0..16_u8 {
                if maps[offset] >> (15 - idx) & 0x0001 == 1 {
                    result.push(mult * 16 + idx)
                };
            }
            offset += 1;
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
fn decode_symbol_map_test() {
    let idx = vec![11008, 32770, 4, 17754, 6208];
    let mut compare = "Making a silly test.".as_bytes().to_vec();
    compare.sort_unstable();
    compare.dedup();
    assert_eq!(compare, decode_sym_map(&idx));
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
