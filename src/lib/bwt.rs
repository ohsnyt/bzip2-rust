use std::cmp::{min, Ordering};

///Burrows-Wheeler-Transform. Probably could be drastically sped up.
/// Receives reference to data. Returns the key as as u32 and the
/// transformed data as a vec of u8.
pub fn bwt_encode(orig: &[u8]) -> (u32, Vec<u8>) {
    // Create index to block.
    let mut index = vec![0; orig.len()];
    for i in 0..index.len() {
        index[i as usize] = i as u32;
    }

    // Sort index (There may be a faster way to do this.)
    index[..].sort_by(|a, b| block_compare(*a as usize, *b as usize, orig));

    // Get key and BWT output. First initialize key and vec to return data
    let mut key: u32 = 0;
    let mut bwt = vec![0; orig.len()];
    // ..and then transform bwt "in place" using the index we built
    for i in 0..bwt.len() {
        if index[i] == 1 {
            key = i as u32;
        }
        if index[i] == 0 {
            bwt[i] = orig[orig.len() - 1]; // wrap around the end of the array
        } else {
            bwt[i] = orig[(index[i] as usize) - 1];
        }
    }
    (key, bwt)
}

/// compare the next two chunks of the original data to decide which sorts first
fn block_compare(a: usize, b: usize, block: &[u8]) -> Ordering {
    let min = min(block[a..].len(), block[b..].len());

    // Lexicographical comparison
    let result = block[a..a + min].cmp(&block[b..b + min]);

    // Implement wraparound if needed
    if result == Ordering::Equal {
        return [&block[a + min..], &block[0..a]]
            .concat()
            .cmp(&[&block[b + min..], &block[0..b]].concat());
    }
    result
}

/// Decode a Burrows-Wheeler-Transform. Requires key and data in. Returns
/// transformed data as vec of u8.
pub fn bwt_decode(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    // First get a freq count of the symbols using an array for speed
    let mut freq = [0; 256];
    for i in 0..bwt_in.len() {
        freq[bwt_in[i] as usize] += 1;
    }
    //then build a cumulative count of frequency counts, again using an array
    let mut sum = 0;
    let mut sum_freqs = [0; 256];
    for i in 0..256 {
        sum_freqs[i] = sum;
        sum += freq[i];
    }
    /*
    Build a transformation vector to find the next character in the original data.
    We know that the original column of the transform was sorted. We can calculate how
    far down that column we need to go by getting the cumulative counts of all u8s that
    came before this one and adding the number of identical u8s to this one that we may
    have previously seen.
    */

    // Re-use the freq count to recount frequencies in the transformation vector
    let mut freq = [0; 256];
    //Build the transformation vector to find the next character in the original data
    let mut t_vec = vec![0; bwt_in.len()];
    for (i, &s) in bwt_in.iter().enumerate() {
        t_vec[sum_freqs[s as usize] + freq[s as usize]] = i;
        freq[s as usize] += 1;
    }
    // Transform the data
    let mut original = vec![0; bwt_in.len()];
    let mut key = key as usize;
    for i in 0..bwt_in.len() {
        original[i] = bwt_in[key];
        key = t_vec[key];
    }
    original
}

#[test]
fn bwt_simple_encode() {
    let input = "How to encrypt using BWT cipher?".as_bytes();
    let output = "gTowtr ?WB n hnpsceitHiyecup  or"
        .to_string()
        .as_bytes()
        .to_vec();
    assert_eq!(bwt_encode(input), (21 as u32, output));
}
#[test]
fn bwt_encode_abracadabra() {
    let input = "abracadabra abracadabra abracadabra".as_bytes();
    let output = "aarrrddda  rrrcccaaaaaaaaaaaabbbbbb"
        .to_string()
        .as_bytes()
        .to_vec();
    assert_eq!(bwt_encode(input), (20 as u32, output));
}

#[test]
fn bwt_simple_decode() {
    let input = "gTowtr ?WB n hnpsceitHiyecup  or".as_bytes().to_vec();
    let output = "How to encrypt using BWT cipher?".as_bytes();
    assert_eq!(output, bwt_decode(21, &input));
}
#[test]
fn bwt_decode_abracadabra() {
    let input = "aarrrddda  rrrcccaaaaaaaaaaaabbbbbb".as_bytes().to_vec();
    let output = "abracadabra abracadabra abracadabra".as_bytes();
    assert_eq!(output, bwt_decode(20, &input));
}


#[test]
fn bwt_encode_decode() {
    let input = "If Peter Piper picked a peck of pickled peppers,  where's the peck of pickled peppers Peter Piper picked????".as_bytes();
    let (key, vec) = bwt_encode(&input);
    let output = bwt_decode(key, &vec);
    assert_eq!(output, input);
}
