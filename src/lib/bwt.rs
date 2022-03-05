use std::{
    cmp::{min, Ordering},
};

///Burrows-Wheeler-Transform | NEEDS WORK. Probably could be drastically sped up.
/// receives reference to incoming block of data and
/// returns key for final data decomcpression. Key is u32.
pub fn bwt_encode(orig: &[u8]) -> (u32, Vec<u8>) {
    // Create index into block. Index is u32, which should be more than enough
    let mut index = vec![0; orig.len()];
    for i in 0..index.len() {
        index[i as usize] = i as u32;
    }
    // Sort index (Is sort by key faster?)
    index[..].sort_by(|a, b| block_compare(*a as usize, *b as usize, orig));

    // Get key and BWT output (assumes u32 is 4 bytes)
    let mut key: u32 = 0;
    let mut bwt = vec![0; orig.len()];
    for i in 0..bwt.len() {
        if index[i] == 0 {
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

/// Decode a Burrows-Wheeler-Transform
//pub fn bwt_decode(key: u32, input: &Vec<u8>, symbols: &[u8]) -> Vec<u8> {
pub fn bwt_decode(key: u32, bwt_in: Vec<u8>) -> Vec<u8> {
    //first get a freq count of symbols
    let mut freq = [0; 256];
    for i in 0..bwt_in.len() {
        freq[bwt_in[i] as usize] += 1;
    }
    //then build a cumulative count of frequency counts
    let mut sum = 0;
    let mut cumulative_offsets = [0; 256];
    for i in 0..256 {
        cumulative_offsets[i] = sum;
        sum += freq[i];
    }
    //Build the transformation vector to find the next character in the original data.
    // We know that the original column of the transform was sorted. We can calculate how
    // far down that column we need to go by getting the cumulative counts of all u8s that
    // came before this one and adding the number of identical u8s to this one that we may
    // have previously seen.
    // Iterate through each element of the input vector and create an index based on the
    // cumulative count of that element. Update the cumulative count so next time we go one more
    let mut t_vec = vec![0; bwt_in.len()];
    for (i, &s) in bwt_in.iter().enumerate() {
        t_vec[cumulative_offsets[s as usize]] = i;
        cumulative_offsets[s as usize] += 1;
    }
    // Transform the data
    let mut orig = vec![];
    let mut key = key as usize;
    for _ in 0..bwt_in.len() {
        key = t_vec[key];
        orig.push(bwt_in[key]);
    }
    println!("\n\n{}", String::from_utf8(bwt_in.clone()).unwrap());
    println!("{}", String::from_utf8(orig.clone()).unwrap());
    orig
}

/*
// I'm going to do this by creating a hashmap to lookup the base u8 offset.
// If you use this together with keeping track of how many similar u8s you
// already found, you can calculate the next index number. Watch below.

// first we need a freq count of symbols. We'll do this as an array.
let mut freq = [0; 256];
for i in 0..input.len() {
    freq[input[i] as usize] += 1;
}

//then build my hashmap
let mut sum = 0;
let offset = symbols.iter().fold(HashMap::new(), |mut hm, &sym| {
    sum += freq[sym as usize];
    hm.insert(sym, sum);
    hm
}); */

/*  // Reset the freq array. "Convert" the key to usize.
freq = [0; 256];
let mut key = key as usize;
let end = input.len();
let mut original = vec![0; end];
let mut counter = 0;
while counter < end {
    let sym = input[key];
    original[key as usize] = sym;
    freq[sym as usize] += 1;
    key = (offset.get(&sym).unwrap() + freq[sym as usize]) % end;
    counter += 1;
}
println!("{}", String::from_utf8(original.clone()).unwrap());
original */

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
    //assert_eq!(output, bwt_decode(21, &input));
}
#[test]
fn bwt_decode_abracadabra() {
    let input = "aarrrddda  rrrcccaaaaaaaaaaaabbbbbb".as_bytes().to_vec();
    let output = "abracadabra abracadabra abracadabra".as_bytes();
    //assert_eq!(output, bwt_decode(20, &input));
}

#[test]
fn bwt_encode_decode() {
    let input = "David Snyder".as_bytes().to_vec();
    let (key, vec) = bwt_encode(&input);
    //let output = bwt_decode(key, &vec);
    //assert_eq!(output, input);
}

fn bwt_encode_classic() {
    let input = "If Peter Piper picked a peck of pickled peppers, where's the peck of pickled peppers Peter Piper picked?????".as_bytes();
    let output = "?fsrrdkkeaddrrffs,es???d\x01 eeiiiieeeehrppkllkp pttpphppPPIootwppppPPcccccckk iipp eeeeeeeeer'ree "
        .to_string()
        .as_bytes()
        .to_vec();
    assert_eq!(bwt_encode(input), (24 as u32, output));
}

fn bwt_decode_classic() {
    let input = "?fsrrdkkeaddrrffs,es???d\x01 eeiiiieeeehrppkllkp pttpphppPPIootwppppPPcccccckk iipp eeeeeeeeer'ree ".as_bytes();
    let output = "If Peter Piper picked a peck of pickled peppers, where's the peck of pickled peppers Peter Piper picked?????"
        .to_string()
        .as_bytes()
        .to_vec();
    assert_eq!(bwt_encode(input), (24 as u32, output));
}