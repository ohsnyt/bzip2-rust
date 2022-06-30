use super::bwt;

///Burrows-Wheeler-Transform - based on https://github.com/aufdj
/// receives reference to incoming block of data and
/// returns key for final data decomcpression. Key is u32.
pub fn bwt_encode(orig: &[u8]) -> (usize, Vec<u8>) {
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
    // Try radix sort
    //rdxsort::RdxSort::rdxsort(&mut index);
    //info!("Known good index: {:?}", index);

    // Get key and BWT output (assumes u32 is 4 bytes)
    let mut key: usize = 0;
    let mut bwt = vec![0; orig.len()];
    for i in 0..bwt.len() {
        if index[i] == 0 {
            key = i as usize;
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

/// Decode a Burrows-Wheeler-Transform using a "super alphabet"
pub fn bwt_decode(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    //  Step 1a: Get a freq count of symbols
    let mut freq = [0_usize; 256];
    for i in 0..bwt_in.len() {
        freq[bwt_in[i] as usize] += 1;
    }
    // Step 1b: Transform the freq count into a cumulative frequency count
    let mut sum = 0_usize;
    freq.iter_mut().for_each(|mut freq| {
        let tmp = *freq;
        *freq = sum;
        sum += tmp
    });

    // Step 2: Build a last-first vector to find the previous character in the original data
    let mut lf = vec![0; bwt_in.len()];
    for (i, &s) in bwt_in.iter().enumerate() {
        lf[freq[s as usize]] = i;
        freq[s as usize] += 1
    }

    // Step 3: Build a last-last (first) vector  of lf[i] and the predecessor to lf[i]
    let mut ll = vec![(0); bwt_in.len() * 2];
    for (i, &s) in bwt_in.iter().enumerate() {
        ll[i * 2] = s;
        ll[i * 2 + 1] = bwt_in[lf[i]];
    }

    // Step 4: Compute last-first 2 vector
    let mut lf2 = vec![0; bwt_in.len()];
    let mut next = lf[0];
    for i in 0..lf.len() {
        lf2[i] = lf[lf[i]];
    }

    // Step 5: Transform the data
    let mut final_data = vec![0_u8; bwt_in.len()];
    let mut key = key as usize;

    let mut l = 0;
    let n = bwt_in.len();

    while l + 1 < n {
        if l == 0 {
            final_data[bwt_in.len() - 1] = ll[2 * key]
        } else {
            final_data[l - 1] = ll[2 * key];
        }
        final_data[l] = ll[2 * key + 1];
        key = lf2[key];
        l += 2;
        if l == n - 1 {
            final_data[l] = ll[2 * key];
        }
    }
    final_data
}

/// Decode a Burrows-Wheeler-Transform using a "super alphabet"
pub fn bwt_decode_mtl(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    //  Step 1a: Get a freq count of symbols
    let mut freq = [0_usize; 256];
    for i in 0..bwt_in.len() {
        freq[bwt_in[i] as usize] += 1;
    }
    // Step 1b: Transform the freq count into a cumulative frequency count
    let mut sum = 0_usize;
    freq.iter_mut().for_each(|mut freq| {
        let tmp = *freq;
        *freq = sum;
        sum += tmp
    });

    // Step 2: Build a last-first vector to find the previous character in the original data
    let mut lf = vec![0; bwt_in.len()];
    for (i, &s) in bwt_in.iter().enumerate() {
        lf[freq[s as usize]] = i;
        freq[s as usize] += 1
    }

    // Step 3: Transform the data
    let mut final_data = vec![0_u8; bwt_in.len()];
    let mut key = key as usize;
    let mut l = 0;
    let n = bwt_in.len();
    key = lf[key];

    while l < n {
        final_data[l] = bwt_in[key];
        key = lf[key];
        l += 1;
    }
    final_data
}

/// Decode a Burrows-Wheeler-Transform using a "super alphabet"
pub fn bwt_decode_orig(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    //  Step 1a: Get a freq count of symbols
    let mut freq = [0_usize; 256];
    for i in 0..bwt_in.len() {
        freq[bwt_in[i] as usize] += 1;
    }
    // Step 1b: Transform the freq count into a cumulative frequency count
    let mut sum = 0_usize;
    freq.iter_mut().for_each(|mut freq| {
        let tmp = *freq;
        *freq = sum;
        sum += tmp
    });

    //Build a transformation vector to find the next character in the original data, but do it by word instead of by byte
    let mut t_vec = vec![0; bwt_in.len()];
    let mut x_vec = vec![0; bwt_in.len()];
    for (i, &s) in bwt_in.iter().enumerate() {
        t_vec[freq[s as usize]] = i;
        x_vec[i] = freq[s as usize];
        freq[s as usize] += 1
    }

    // Transform the data
    let mut orig = vec![0; bwt_in.len()];
    let mut key = t_vec[key as usize];

    //for i in 0..bwt_in.len() {
    for item in orig.iter_mut().take(bwt_in.len()) {
        *item = bwt_in[key];
        key = t_vec[key]
    }
    orig
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
