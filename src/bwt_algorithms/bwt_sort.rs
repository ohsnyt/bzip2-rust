use super::sais_fallback::sais_entry;
use crate::tools::freq_count::freqs;
use log::{info, warn};
use rayon::prelude::*;
/*
I tried a varient that uses a double length block to avoid the nested equality checks
in block_compare, but it was barely faster.
*/

/// Burrows-Wheeler-Transform using Rayon to multi-thread. We check for possibly repetative data like found
/// in genetic sequences. For that data we use a SA-IS algorithm. For other data we use a native
/// sort_unstable algorithm.
/// This returns a u32 Key and a u8 vec of the BWT data.
pub fn bwt_encode(rle1_data: &[u8]) -> (u32, Vec<u8>) {
    // Create index into block. Index is u32, which should be more than enough
    let mut index = (0_u32..rle1_data.len() as u32).collect::<Vec<u32>>();
    // Run a repetative data test for data longer than 2k bytes
    /*
    NOTE: Currently testing for the number of different bytes in 2k of data. This isn't
    really a great test, but it is fast and does focus SAIS on genetic type data.
     */

    if rle1_data.len() < 3_000 || use_sais(&rle1_data[0..5_000.min(rle1_data.len())]) {
        info!("Using SA-IS algorithm.");
        return sais_entry(rle1_data);
    }

    info!("Using native algorithm.");

    // Sort index
    if rle1_data.len() > 40000 {
        index[..].par_sort_unstable_by(|a, b| block_compare(*a as usize, *b as usize, rle1_data));
    } else {
        index[..].sort_unstable_by(|a, b| block_compare(*a as usize, *b as usize, rle1_data));
    }
    // Get key and BWT output (assumes u32 is 4 bytes)
    let mut key = 0_u32;
    let mut bwt = vec![0; rle1_data.len()];
    for i in 0..bwt.len() {
        if index[i] == 0 {
            key = i as u32;
        }
        if index[i] == 0 {
            bwt[i] = rle1_data[rle1_data.len() - 1];
        } else {
            bwt[i] = rle1_data[(index[i] as usize) - 1];
        }
    }
    // println!("Key is: {}", key);
    // println!("BWT is: {:?}", bwt);
    (key, bwt)
}

/// compare the next two chunks of the original data to decide which sorts first
fn block_compare(a: usize, b: usize, block: &[u8]) -> std::cmp::Ordering {
    let min = std::cmp::min(block[a..].len(), block[b..].len());

    // Lexicographical comparison
    let mut result = block[a..a + min].cmp(&block[b..b + min]);

    // Implement wraparound if needed
    if result == std::cmp::Ordering::Equal {
        if a < b {
            let to_end = block.len() - a - min;
            result = block[(a + min)..].cmp(&block[..to_end]);
            if result == std::cmp::Ordering::Equal {
                let rest_of_block = block.len() - to_end - min;
                return block[..rest_of_block].cmp(&block[to_end..(to_end + rest_of_block)]);
            }
        } else {
            let to_end = block.len() - b - min;
            result = block[..to_end].cmp(&block[(b + min)..]);
            if result == std::cmp::Ordering::Equal {
                let rest_of_block = block.len() - to_end - min;
                return block[to_end..(to_end + rest_of_block)].cmp(&block[..rest_of_block]);
            }
        }
    }
    result
}

/// Decode a Burrows-Wheeler-Transform. All variations seem to have excessive cache misses.
pub fn bwt_decode(key: u32, bwt_in: &[u8], freq_in: &[u32]) -> Vec<u8> {
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
    let mut rle1_data = vec![0_u8; end];
    for i in 0..bwt_in.len() {
        rle1_data[i] = bwt_in[keys[i] as usize] as u8;
    }

    rle1_data
}

fn use_sais(data: &[u8]) -> bool {
    // Use sais if the most frequent char is more than 30% of all chars found
    //   or if the symbol count is less than 20 unique symbols
    let mut freq_array = freqs(data);
    freq_array.retain(|&x| x != 0);
    warn!(
        "Max frequency is {}%, symbol set size is {}.  ",
        (*freq_array.iter().max().unwrap() * 10) / data.len() as u32,
        freq_array.len()
    );
    if (*freq_array.iter().max().unwrap() * 10) / data.len() as u32 != 1 || freq_array.len() < 20 {
        warn!("Using SA-IS");
        return true;
    }

    // Use sais if the longest run is > 20% of the length
    let mut longest = 0;
    let mut run = 0;
    for i in 1..data.len() {
        if data[i - 1] == data[i] {
            run += 1;
        } else {
            if run > longest {
                longest = run;
            }
            run = 0;
        }
    }
    warn!("Longest is {}.  ", longest);
    if longest * 10 / data.len() > 2 {
        warn!("Using SA-IS");
    } 

    longest * 10 / data.len() > 2
}
