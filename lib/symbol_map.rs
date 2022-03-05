/// Similar to makeMaps_e
/// Takes a sorted deduped vec of all symbols used in the input and
/// creates the unique bzip2 symbol map of symbols encoded as u16s
pub fn encode_sym_map(symbols: &[u8]) -> Vec<u16> {
    let mut map_l1: u16 = 0;
    let mut map_l2: Vec<u16> = vec![0; 16];

    for byte in symbols.iter() {
        let l1 = byte / 16; // find out which map set the bit mask belongs to
        map_l1 |= 0x8000 >> l1; //mask the appropriate set
        map_l2[l1 as usize] |= 0x8000 >> (byte % 16); //index to the set and set the mask there
    }
    // Get ready to return only those vecs that have bits set.
    let mut sym_vec: Vec<u16> = vec![map_l1];
    for map in map_l2 {
        if map > 0 {
            sym_vec.push(map)
        }
    }
    sym_vec
}

/// Similar to ???
/// Takes the unique bzip2 symbol map and returns a sorted vec of all
/// symbols used in the input as u8s
pub fn decode_sym_map(maps: &[u16]) -> Vec<u8> {
    let map_l1: u16 = maps[0];

    let mut result: Vec<u8> = vec![];
    let mut offset = 1;
    for mult in 0..16 {
        if map_l1 >> (15 - mult) & 0x0001 == 1 {
            for idx in 0..16 {
                if maps[offset] >> (15 - idx) & 0x0001 == 1 {
                    result.push((mult) as u8 * 16 + idx as u8)
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
    let idx = vec![22016, 32770, 4, 17754, 6208];
    assert_eq!(idx, encode_sym_map(&x))
}
#[test]
fn decode_symbol_map_test() {
    let idx = vec![22016, 32770, 4, 17754, 6208];
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
    let rt = encode_sym_map(&x);
    assert_eq!(decode_sym_map(&rt), x)
}