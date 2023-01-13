use rayon::prelude::*;

///Burrows-Wheeler-Transform. Uses rayon to multi-thread.
/// Transforms a u8 sli&ce using bwt. The key is u32.
pub fn bwt_encode(orig: &[u8]) -> (u32, Vec<u8>) {
    // Create index into block. Index is u32, which should be more than enough
    let mut index = vec![0; orig.len()];
    for i in 0..index.len() {
        index[i as usize] = i as u32;
    }

    // Sort index (par_sort_by is 3x faster than .sort_by)
    if orig.len() > 40000 {
        index[..].par_sort_unstable_by(|a, b| block_compare(*a as usize, *b as usize, orig));
    } else {
        index[..].sort_unstable_by(|a, b| block_compare(*a as usize, *b as usize, orig));
    }
    // Tried radix sort, but it was slower
    //rdxsort::RdxSort::rdxsort(&mut index);
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
pub fn bwt_decode_test(key: u32, bwt_in: &[u8], freq_in: &[u32]) -> Vec<u8> {
    // Calculate end once.
    let end = bwt_in.len();

    // Convert frequency count to a cumulative sum of frequencies
    let mut freq = [0_u32; 256];

    {
        for i in 0..255 {
            freq[i + 1] = freq[i] + freq_in[i];
        }
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
    let key = key;

    // Assign to keys[0] to avoid a temporary assignment below
    keys[0] = t_vec[key as usize];

    for i in 1..end {
        keys[i] = t_vec[keys[i - 1] as usize];
    }

    // Transform the data
    let mut orig = vec![0_u8; end];
    for i in 0..bwt_in.len() {
        orig[i] = bwt_in[keys[i] as usize] as u8;
    }

    orig
}
