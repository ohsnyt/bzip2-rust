///Burrows-Wheeler-Transform
/// Transforms a u8 sli&ce using bwt. The key is u32.
pub fn bwt_encode(orig: &[u8]) -> (u32, Vec<u8>) {
    // Create index into block. Index is u32, which should be more than enough
    let mut index = vec![0; orig.len()];
    for i in 0..index.len() {
        index[i as usize] = i as u32;
    }
    // Sort index
    index[..].sort_by(|a, b| block_compare(*a as usize, *b as usize, orig));
    // Tried radix sort, but it was slower
    //rdxsort::RdxSort::rdxsort(&mut index);
    //info!("Known good index: {:?}", index);

    // Get key and BWT output (assumes u32 is 4 bytes)
    let mut key = 0_u32;
    let mut bwt = vec![0; orig.len()];
    for i in 0..bwt.len() {
        if index[i] == 0 {
            key = i as u32;
        }
        if index[i] == 0 {
            bwt[i] = orig[orig.len() - 1];
        } else {
            bwt[i] = orig[(index[i] as usize) - 1];
        }
    }
    (key, bwt)
}

/// compare the next two chunks of the original data to decide which sorts first
fn block_compare(a: usize, b: usize, block: &[u8]) -> std::cmp::Ordering {
    let min = std::cmp::min(block[a..].len(), block[b..].len());

    // Lexicographical comparison
    let result = block[a..a + min].cmp(&block[b..b + min]);

    // Implement wraparound if needed
    if result == std::cmp::Ordering::Equal {
        return [&block[a + min..], &block[0..a]]
            .concat()
            .cmp(&[&block[b + min..], &block[0..b]].concat());
    }
    result
}

/// Decode a Burrows-Wheeler-Transform. All variations seem to have excessive cache misses.
pub fn bwt_decode_fastest(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    // Calculate end once.
    let end = bwt_in.len();

    // Use u32 instead of usize to keep memory needs down.
    // First get a freq count of symbols.
    let mut freq = vec![0_u32; 256];
    for i in 0..end {
        freq[bwt_in[i] as usize] += 1;
    }
    let mut sum = 0;

    // This is slightly faster than iter_mut().for_each
    for i in 0..256 {
        let tmp = freq[i];
        freq[i] = sum;
        sum += tmp;
    }

    //Build the transformation vector to find the next character in the original data
    // Using an array instead of a vec saves about 4 ms.
    let mut t_vec = [0_u32; 900024];
    for (i, &s) in bwt_in.iter().enumerate() {
        t_vec[freq[s as usize] as usize] = i as u32;
        freq[s as usize] += 1
    }

    //Build the keys vector to find the next character in the original data
    // This is the slowest portion of this function - I assume cache misses causes problems
    // It slows down when t_vec is over about 500k.
    let mut keys = vec![0_u32; end];
    let mut key = key;

    // Assign to keys[0] to avoid a temporary assignment below
    keys[0] = t_vec[key as usize];

    for i in 1..end {
        keys[i] = t_vec[keys[i - 1] as usize];
    }

    // Transform the data
    let mut orig = vec![0; end];
    for i in 0..bwt_in.len() {
        orig[i] = bwt_in[keys[i] as usize];
    }
    orig
}

/// Decode a Burrows-Wheeler-Transform. All variations seem to have excessive cache misses.
pub fn bwt_decode_test(key: u32, bwt_in: &[u32], mut freq_in: [u32; 256]) -> Vec<u8> {
    // Calculate end once.
    let end = bwt_in.len();

    // Convert frequency count to a cumulative sum of frequencies
    let mut freq = [0_u32; 256];
    {
        for i in 0..255 {
            freq[i + 1] = freq[i] + freq_in[i];
        }
    }

    // Faster decompression algorithm
    /* Compute the T^(-1) vector
    Each element of the input block is used to compute the index of the suffix to that symbol.
    The suffix pointer (24 bits) is combined with the element (8 bits) to create a u32 combined
    info word for the t^(-1) vector (24 bits of pointer and 8 bits of symbol).
    */
    let mut t_vec = vec![0_u32; end + 19];

    for (i, &s) in bwt_in.iter().enumerate() {
        t_vec[freq[s as usize] as usize] = ((i as u32) << 8 | s);
        freq[s as usize] += 1;
    }

    // Transform the data. Initialize the output vec.
    let mut orig = vec![0_u8; end];

    // Get the origin key and use it to get first element key.
    let mut key = key as usize;
    //let (k, _) = key_byte(t_vec[key]);
    //key = k;

    for el in orig.iter_mut() {
        let (k, b) = key_byte(t_vec[key]);
        key = k;
        *el = b;
    }

    orig
}

fn key_byte(complex: u32) -> (usize, u8) {
    let byte = (complex & 0xff) as u8;
    let b2 = complex as u8;
    let ptr = complex >> 8;
    ((complex >> 8) as usize, (complex & 0xff) as u8)
}

/// Decode a Burrows-Wheeler-Transform
pub fn bwt_decode(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    // First get a freq count of symbols
    let mut freq = vec![0_usize; 256];
    for i in 0..bwt_in.len() {
        freq[bwt_in[i] as usize] += 1;
    }
    let mut sum = 0;

    // This is slightly faster than iter_mut().for_each
    for i in 0..256 {
        let tmp = freq[i];
        freq[i] = sum;
        sum += tmp;
    }

    //Build the transformation vector to find the next character in the original data
    let mut t_vec = vec![0; bwt_in.len()];
    for (i, &s) in bwt_in.iter().enumerate() {
        t_vec[freq[s as usize]] = i;

        freq[s as usize] += 1
    }

    // Transform the data
    let mut orig = vec![0; bwt_in.len()];
    let mut key = t_vec[key as usize];

    // for i in 0..bwt_in.len() {
    //     orig[i] = bwt_in[key];
    //     key = t_vec[key]
    // }
    for el in orig.iter_mut().take(bwt_in.len()) {
        *el = bwt_in[key];
        key = t_vec[key]
    }
    orig
}

/// Decode a Burrows-Wheeler-Transform, cache interleaved (oddly slower)
pub fn bwt_decode_new(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    // First get a freq count of symbols
    let mut freq = vec![0_usize; 256];
    for el in bwt_in {
        freq[*el as usize] += 1;
    }
    let mut sum = 0;

    // This is slightly faster than iter_mut().for_each
    for i in 0..256 {
        let tmp = freq[i];
        freq[i] = sum;
        sum += tmp;
    }

    //Build the last-first vector
    let mut lf = vec![0; bwt_in.len()];
    for i in 0..lf.len() {
        lf[freq[bwt_in[i] as usize]] = i;
        freq[bwt_in[i] as usize] += 1;
    }

    // Build the last-last vector
    let mut ll = vec![0; bwt_in.len() * 2];
    for i in 0..bwt_in.len() {
        ll[2 * i] = bwt_in[i];
        ll[2 * i + 1] = bwt_in[lf[i]];
    }

    // Build the lf2 vector
    let mut lf2 = vec![0; bwt_in.len()];
    for i in 0..lf2.len() {
        lf2[i] = lf[lf[i]];
    }

    // Fix the first key
    let mut key = key as usize;

    // Transform the data
    let mut original = vec![0; bwt_in.len()];
    let mut i = 0;
    while i < original.len() {
        original[i] = ll[(2 * key)];
        if i + 1 < original.len() {
            original[i + 1] = ll[(2 * key + 1)]
        }
        key = lf2[key];
        i += 2;
    }
    original
}

#[test]
fn bwt_simple_decode() {
    let input = "gTowtr ?WB n hnpsceitHiyecup  or".as_bytes().to_vec();
    let output = "How to encrypt using BWT cipher?".as_bytes();
    assert_eq!(output, bwt_decode(8, &input));
}
#[test]
fn bwt_decode_abracadabra() {
    let input = "aarrrddda  rrrcccaaaaaaaaaaaabbbbbb".as_bytes().to_vec();
    let output = "abracadabra abracadabra abracadabra".as_bytes();
    assert_eq!(output, bwt_decode(9, &input));
}
