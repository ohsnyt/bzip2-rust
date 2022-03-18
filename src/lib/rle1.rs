/*
Logic: Iterate through the input, counting how far we go before we hit a sequence of 4 identical bytes.
(Extending a vec should be faster than pushing data at each byte. Searching for 4 bytes at a time should
be faster than counting pairs in a loop.) When you find a duplicate sequence, output all the bytes
 you skipped over, go count how many bytes are identical, output the identical bytes (you can only
do 260 at a time, hence the divide and mod math), then adjust the index and start location.*/

/// Performs RLE encoding on strings of four or more identical characters.
pub fn rle1_encode(v: &[u8]) -> Vec<u8> {
    // Initialize several variables
    // skip_start marks where we started skipping bytes while we look for the next sequence
    let mut skip_start: usize = 0;
    // idx is the index of where we are in the input file, starting from 0 to the end.
    let mut idx = 0;
    // the end is three bytes from the end, because... we are looking for sequences of 4 bytes!
    let end = v.len().saturating_sub(3);
    // out is the vec holding the transform - it will be the same size as the input
    let mut out = Vec::with_capacity(v.len());

    // Loop through the input file looking for a sequence of four identical bytes
    while idx < end {
        if v[idx] == v[idx + 1] && v[idx] == v[idx + 2] && v[idx] == v[idx + 3] {
            // If we found a sequence, first push out all that we skipped over to get here
            out.extend_from_slice(&v[skip_start..idx]);
            // Go count how many identical bytes start at the index (4+, but how many?)
            let mut dups = count_dups(v, idx);
            // Since we can only write out 260 at a time, write out the duplicates in groups as needed
            for _ in 0..dups / 260 {
                // each time we write the 4 bytes we found...
                out.extend_from_slice(&v[idx..=idx + 3]);
                // ...followed by a number that says how many more should be there, up to 255
                out.push(255);
                // Then reduce the dups counter
                dups -= 260;
            }
            // When we got down to less than 260, write out the rest
            // Write the 4 bytes we found...
            out.extend_from_slice(&v[idx..=idx + 3]);
            // ...followed by a number that says how many more should be there
            out.push(((dups) % 256) as u8);
            // move the index past the sequence we found
            idx += dups + 4;
            // and reset the skip_start to the same point
            skip_start = idx;
            // dups gets reset above, so we can leave it for now
        }
        idx += 1;
    }

    // We probably didn't finish with a sequence of identical bytes, so write out all we skipped
    // since the last sequence (or the beginning, if we didn't have any sequences)
    if skip_start < v.len() {
        out.extend_from_slice(&v[skip_start..v.len()]);
    }
    out
}

/*
Logic: This is similar to the encoding. Start looking for a sequence of 4 identical bytes.
When you find them, get the next byte, which is a count of how many more such bytes are needed.
First output everthing from the start until the end of the sequence we found (not counting the
count byte) followed by a vec created with the repeating byte we want to insert. Since we want
to get past the 4 bytes plus the counter that we found, put 5 into a jump_past_search variable
and loop until that decrements to zero.
Perhaps I should have use the while idx strategy rather than a for loop, but...
*/
/// Unencodes runs of four or more characters from the RLE1 phase
pub fn rle1_decode(v: &[u8]) -> Vec<u8> {
    // Initialize our start counter and jump_past_search counter
    let mut start: usize = 0;
    let mut jump_past_search = 0;
    // Create a vec with the same capacity as the input
    let mut out = Vec::with_capacity(v.len());
    // loop until we get within 4 bytes of the end
    for i in 0..v.len() - 4 {
        // If we found a sequence of 4 identical bytes, we need to get past it
        if jump_past_search > 0 {
            // decrement the jump counter by one
            jump_past_search -= 1;
            // If not, look for a sequence of 4
        } else if v[i] == v[i + 1] && v[i] == v[i + 2] && v[i] == v[i + 3] {
            // If we found a sequence of 4, first write out everything from start to now.
            out.extend_from_slice(&v[start..i + 4]);
            // Create a vec of the identical characters, sizing the vec on the counter byte 
            // after sequence of 4 we found
            let tmp = vec![v[i]; v[i + 4].into()];
            // ...and write that out
            out.extend(tmp);
            // Set the jump past variable to get us past the 4 plus count byte
            jump_past_search = 5;
            // and reset the start to be at the point just past the jump_past counter.
            start = i + jump_past_search;
        }
    }
    // Don't forget to write out any stuff we have skipped since the last start counter
    out.extend_from_slice(&v[start..v.len()]);
    // Return the transformed data
    out
}

/// Helper function for rel1_encode to count how many duplicate bytes occur.
fn count_dups(v: &[u8], i: usize) -> usize {
    // Initialize a counter
    let mut count = 0;
    // Starting three past the current index location
    for j in i + 3..v.len() - 1 {
        // If we have a different byte, return the count
        if v[j] != v[j + 1] {
            return count;
        }
        // Otherwise increment the count an check the next byte
        count += 1;
    }
    // return the count, needed in case we end the loop without finding a different byte
    count
}

#[test]
fn rle1_en_simple() {
    let input = "Goofy teeeeeeeest".as_bytes();
    assert_eq!(
        rle1_encode(input),
        vec![71, 111, 111, 102, 121, 32, 116, 101, 101, 101, 101, 4, 115, 116]
    )
}
#[test]
fn rle1_en_simple2() {
    let input = "Goofy teeeeeeeest".as_bytes();
    assert_eq!(
        rle1_encode(input),
        vec![71, 111, 111, 102, 121, 32, 116, 101, 101, 101, 101, 4, 115, 116]
    )
}
#[test]
fn rle1_roundtrip_simple() {
    let input =
        "Peter Piper              picked a biiiiiiiiiiiiig peck of peppersssssss".as_bytes();
    let e = rle1_encode(input);
    let d = rle1_decode(&e);
    assert_eq!(input, d);
}

#[test]
fn rle1_de_simple() {
    let input: Vec<u8> = vec![
        71, 111, 111, 102, 121, 32, 116, 101, 101, 101, 101, 4, 115, 116,
    ];
    assert_eq!(rle1_decode(&input), "Goofy teeeeeeeest".as_bytes());
}
