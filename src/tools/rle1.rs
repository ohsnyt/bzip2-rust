/*
Stream orient Run Length Encoder. Being stream oriented allows this encoder to work with
the logic that divides a block so that the BWT encoding can received as close to the
maximum block size as possible without overrunning the 19 byte buffer.

This maximum block size must be considered for both the data BWT will see as well as the
data the RLE1 output will receive. Remember, while RLE1 output is usually smaller than the BWT
data, it can be as much as 25% larger than the BWT data.

Logic: Iterate through the input, counting how far we go before we hit a sequence of 4 identical bytes.
(Extending a vec is faster than pushing data at each byte. Searching for 4 bytes at a time is
be faster than counting pairs in a loop.) When you find a duplicate sequence, output all the bytes
 you skipped over, go count how many bytes are identical, output the identical bytes (you can only
do 260 at a time, hence the divide and mod math), then adjust the index and start location.*/

/// Encode runs of for our more identical bytes, pre-BWT. Returns how many bytes we used and the RLE1 data.
pub fn rle_encode(data: &[u8], free_space: usize) -> (usize, Vec<u8>) {
    // Skip RLE1 if the data is less than 4 bytes
    if data.len() < 4 {
        return (data.len(), data.to_vec());
    }

    // Pushing a byte at a time to the output is slower than extending by chunks
    // Therefore remember where we are starting the next chunk so we can go faster
    let mut start: usize = 0;

    // It is possible that the data will expand a maximum of 25%
    let mut out = Vec::with_capacity(data.len() * 5 / 4);

    // idx is the position in the input where we are looking for 3 identical *previous* bytes to this one
    let mut idx = 3;

    // As long as we have data, search
    while idx < data.len() {
        // Remembering that RLE1 can possibly expand the data, watch out that we don't make our blocks too big
        if (out.len() + idx - start) >= free_space {
            break;
        }

        // Look at the next 4 bytes to see if they are identical
        if data[idx] == data[idx - 1] && data[idx] == data[idx - 2] && data[idx] == data[idx - 3] {
            // If they are, write out everything from the last start until the end of the run of 4
            out.extend_from_slice(&data[start..=idx]);
            // Get the count of duplicates following (0-255)
            let dups = count_dups(data, idx);
            // Write that out
            out.push(dups);
            // Move the index past the 4 identical bytes and the duplicates we counted
            idx += 1 + dups as usize;
            // Reset start to this new index
            start = idx;
        } else {
            // Otherwise just increment the index
            idx += 1;
        }
    }

    // Write out any pending data.
    out.extend_from_slice(&data[start..idx]);

    // Report how much of the input was processed and return the output
    (idx, out)
}

/// Helper function for rel1_encode to count how many duplicate bytes occur.
fn count_dups(data: &[u8], i: usize) -> u8 {
    // i is the position of the last of four identical bytes. We need to count
    // identical bytes *after* that position. If position returns None, then return
    // the number of bytes to the end of the data we have taken.
    data.iter()
        .skip(i + 1)
        .take(255)
        .position(|&x| x != data[i])
        .unwrap_or(data.len() - i - 1) as u8
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
pub fn rle1_decode(data: &[u8]) -> Vec<u8> {
    // Initialize our start counter and jump_past_search counter
    let mut start: usize = 0;
    // Create a vec with 10% more capacity as the input
    let mut out = Vec::with_capacity(data.len() * 11 / 10);
    // loop until we get within 4 bytes of the end
    for i in 0..data.len() - 4 {
        // If we found a sequence of 4 identical bytes, we need to get past it
        if i < start {
            continue;
            // If not, look for a sequence of 4
        } else if data[i] == data[i + 1] && data[i] == data[i + 2] && data[i] == data[i + 3] {
            // If we found a sequence of 4, first write out everything from start to now.
            out.extend_from_slice(&data[start..i + 4]);
            // Create a vec of the identical characters, sizing the vec on the counter byte
            // after sequence of 4 we found
            let tmp = vec![data[i]; data[i + 4].into()];
            // ...and write that out
            out.extend(tmp);
            // and reset the start to be at the point just past the jump_past counter.
            start = i + 5;
        }
    }
    // Don't forget to write out any stuff we have skipped since the last start counter
    out.extend_from_slice(&data[start..data.len()]);
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
