/*
Logic: Iterate through the input, counting how far we go before we hit a sequence of 4 identical bytes.
(Extending a vec should be faster than pushing data at each byte. Searching for 4 bytes at a time should
be faster than counting pairs in a loop.) When you find a duplicate sequence, output all the bytes
 you skipped over, go count how many bytes are identical, output the identical bytes (you can only 
do 260 at a time, hence the divide and mod math), then adjust the index and start location.*/

/// Performs RLE encoding on strings of four or more identical characters.
/// (Q: Can it go away during encode? Should only be needed on legacy bz2 files.)
pub fn rle1_encode(v: &[u8]) -> Vec<u8> {
    let mut skip_start: usize = 0;
    let mut idx = 0;
    let mut out = Vec::with_capacity(v.len());
    let end = v.len().saturating_sub(3);
    while idx < end {
        if v[idx] == v[idx + 1] && v[idx] == v[idx + 2] && v[idx] == v[idx + 3] {
            out.extend_from_slice(&v[skip_start..idx]);
            let dups = count_dups(v, idx); 
            for _ in 0..dups / 260 {
                out.extend_from_slice(&v[idx..=idx + 3]); 
                out.push(255); 
            }
            out.extend_from_slice(&v[idx..=idx + 3]); 
            out.push(((dups) % 256) as u8);
            idx += dups + 4; 
            skip_start = idx; 
        }
        idx += 1;
    }

    // If needed, write what we skipped at the end
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
    let mut start: usize = 0;
    let mut jump_past_search = 0;
    let mut out = vec![];
    for i in 0..v.len() - 4 {
        if jump_past_search > 0 {
            jump_past_search -= 1;
        } else if v[i] == v[i + 1] && v[i] == v[i + 2] && v[i] == v[i + 3] {
            out.extend_from_slice(&v[start..i + 4]);
            let tmp = vec![v[i]; v[i + 4].into()];
            out.extend(tmp);
            jump_past_search = 5;
            start = i + jump_past_search;
        }
    }
    out.extend_from_slice(&v[start..v.len()]);
    out
}

/// Helper function for rel1_encode to count how many duplicate bytes occur. 
fn count_dups(v: &[u8], i: usize) -> usize {
    let mut count = 0;
    for j in i + 3..v.len() - 1 {
        if v[j] != v[j + 1] {
            return count;
        }
        count += 1;
    }
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
