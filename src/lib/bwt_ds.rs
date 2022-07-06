use std::time::Instant;

///Burrows-Wheeler-Transform
/// Transforms a u8 sli&ce using bwt. The key is u32.
pub fn bwt_encode(orig: &[u8]) -> (u32, Vec<u8>) {
    // Create index into block. Index is u32, which should be more than enough
    //let ext = orig.len();
    //let mut index: Vec<(u8, usize)> = orig.iter().enumerate().map(|(i, &s)| (s, i)).collect();
    //index.append(&mut (orig.iter().enumerate().map(|(i, &s)| (s, i + ext)).collect()));

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
    let mut keys = vec![0_u32; end];
    let mut key = key;

    // Assign to vec[0] to avoid a temporary assignment below
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
pub fn bwt_decode_small(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    let mut time = Instant::now();
    // Calculate end once.
    let end = bwt_in.len();

    // Use u32 instead of usize to keep memory needs down.
    // First get a freq count of symbols.
    let mut freq = vec![0_u32; 256];

    for i in 0..end {
        freq[bwt_in[i] as usize] += 1;
    }
    let freqa = time.elapsed().as_micros();
    println!("Freq a: {} µs", freqa);

    let mut sum = 0;
    
    // This is slightly faster than iter_mut().for_each
    for i in 0..256 {
        let tmp = freq[i];
        freq[i] = sum;
        sum += tmp;
    }
    let freqb = time.elapsed().as_micros();
    println!("Freq b: {} µs", freqb - freqa);

    //Build the transformation vector to find the next character in the original data
    // Using an array instead of a vec saves about 4 ms.
    // The t_vec numbers are somewhat grouped
    let mut t_veca = vec![0_u32; end / 2];
    let mut t_vecb = vec![0_u32; end - end / 2];
    for (i, &s) in bwt_in.iter().enumerate() {
        let tmp = freq[s as usize] as usize;
        if tmp < (end / 2) {
            t_veca[tmp] = i as u32;
        } else {
            t_vecb[tmp - (end / 2)] = i as u32;
        }
        freq[s as usize] += 1
    }
    let tvec = time.elapsed().as_micros();
    println!("t_vec: {} µs", tvec - freqb);

    // Build the keys vector to find the next character in the original data.
    // (It is faster to do this as a separate step from the transformation.)
    // Building the keys vec is the slowest portion of this function. The keys are widely
    // scattered numerically (0-900k). I assume cache misses causes the speed problem here.
    let mut keys = vec![0_u32; end];
    let mut key = key;

    // Assign to vec[0] to avoid a temporary assignment below
    if key < (end as u32) / 2 {
        keys[0] = t_veca[key as usize]
    } else {
        keys[0] = t_vecb[key as usize - end / 2]
    }

    for i in 1..end {
        let tmp = keys[i - 1];
        if key < (end as u32) / 2 {
            keys[i] = t_veca[key as usize]
        } else {
            keys[i] = t_vecb[key as usize - end / 2]
        }
    }
    let key_time = time.elapsed().as_micros();
    println!("keys: {} µs", key_time - tvec);

    // Transform the data
    let mut orig = vec![0; end];
    for i in 0..bwt_in.len() {
        orig[i] = bwt_in[keys[i] as usize];
    }
    let t_time = time.elapsed().as_micros();
    println!("transform: {} µs", t_time - key_time);
    println!("Total: {:?}", time.elapsed());

    orig
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

    for i in 0..bwt_in.len() {
        orig[i] = bwt_in[key];
        key = t_vec[key]
    }
    orig
}

/// Decode a Burrows-Wheeler-Transform, cache interleaved (oddly slower)
pub fn bwt_decode_new(key: u32, bwt_in: &[u8]) -> Vec<u8> {
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
