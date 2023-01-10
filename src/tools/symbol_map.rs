const BIT_MASK: u16 = 0x8000;


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
    let mut symbols: Vec<u8> = Vec::with_capacity(256);
    // Set a counter for the number of maps
    let mut map_idx = 0;

    for block in 0..16 {
        // Check the index to see if the next bit has a block of bytes
        if (symbol_map[0] & (BIT_MASK >> block)) > 0 {
            // Found one, so increment the index to the correct symbol map offset
            map_idx += 1;
            // Within that u16, iterate to find which bytes were present
            for byte_idx in 0..16_u8 {
                // Is the next bit set (indicating that symbol existed in the data)?
                if (symbol_map[map_idx] & (BIT_MASK >> byte_idx)) > 0 {
                    // Store this symbol on the symbols vec. (block * 16 + byte_idx = u8 value we found)
                    symbols.push((block << 4) + byte_idx);
                };
            }
        }
    }
    symbols
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
    let compare = (0..=255).collect::<Vec<u8>>();
    assert_eq!(compare, decode_sym_map(&maps));
}
