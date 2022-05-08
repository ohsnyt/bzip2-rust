/*
Stream orient Run Length Encoder. Being stream oriented allows this encoder to work with
the logic that divides a block so that the BWT encoding can received as close to the
maximum block size as possible without overrunning the 19 byte buffer.

Logic: Iterate through the input, counting how far we go before we hit a sequence of 4 identical bytes.
(Extending a vec should be faster than pushing data at each byte. Searching for 4 bytes at a time should
be faster than counting pairs in a loop.) When you find a duplicate sequence, output all the bytes
 you skipped over, go count how many bytes are identical, output the identical bytes (you can only
do 260 at a time, hence the divide and mod math), then adjust the index and start location.*/

/// Encode runs of for our more identical bytes, pre-BWT. Returns number of bytes consumed and the RLE1 data. 
pub fn rle_encode(v: &[u8], size: usize) -> (usize, Vec<u8>) {
    if v.len() < 4 || size < 4 {
        return (v.len().min(size), v.to_vec());
    }
    let mut skip_start: usize = 0;
    let mut idx = 0;
    let mut out = Vec::with_capacity(v.len());

    let data_end = v.len() - 3;
    let mut consumed_dups = 0;
    let mut idx = 0;

    while idx < data_end  {
        if out.len() + (idx - skip_start) >= size  {
            break
        }
        if v[idx] == v[idx + 1] && v[idx] == v[idx + 2] && v[idx] == v[idx + 3] {
            out.extend_from_slice(&v[skip_start..idx]);
            let dups = count_dups(v, idx);
            out.extend_from_slice(&v[idx..=idx + 3]);
            out.push(dups as u8);
            idx += dups + 4;
            skip_start = idx;
            consumed_dups += dups;
        } else {
            idx += 1;
        }
    }
    // If we are nearly at the end of the data, fix idx
    if v.len() - idx <= 3 || v.len() < idx {
        idx = v.len()
    }

    // If needed, write what we skipped
    if skip_start < v.len() {
        out.extend_from_slice(&v[skip_start..idx]);
    }
    (idx, out)
}

/// Helper function for rel1_encode to count how many duplicate bytes occur.
fn count_dups(v: &[u8], i: usize) -> usize {
    let mut count = 0;
    for j in i + 3..v.len() - 1 {
        if v[j] != v[j + 1] {
            return count;
        }
        count += 1;
        if count == 255 {
            break;
        }
    }
    count
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

#[test]
fn rle1_de_simple() {
    let input: Vec<u8> = vec![
        71, 111, 111, 102, 121, 32, 116, 101, 101, 101, 101, 4, 115, 116,
    ];
    assert_eq!(rle1_decode(&input), "Goofy teeeeeeeest".as_bytes());
}
