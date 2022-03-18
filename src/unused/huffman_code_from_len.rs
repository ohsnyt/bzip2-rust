/// Create a vec of Huffman codes (u32) from a vec of symbol lengths (u8).
fn code_from_len(list: Vec<(u8, u32)>) -> Vec<(u8, u32)> {
    // We should never get an empty input, but... just in case
    if list.is_empty() {
        return vec![];
    }

    // Copy the list of lengths so we can sort it by length
    let mut list_by_len = list.clone();
    list_by_len.sort_by(|(_, l1), (_, l2)| l1.cmp(l2));

    // Get the minimum length in use so we can start at the first entry
    let mut last_code: (u32, u32) = (list[0].1, 0);

    // Create a vec that we can push to so we can return the codes.
    let mut codes = vec![];

    // For each code (sorted by length), increment the code by one. When the length changes, do a shift
    // left for each increment and continue.
    for (sym, len) in &list_by_len {
        if *len != last_code.0 {
            last_code.1 <<= len - last_code.0;
            last_code.0 = *len;
        }
        codes.push((*sym, last_code.1));
        last_code.1 += 1;
    }
    codes
}
