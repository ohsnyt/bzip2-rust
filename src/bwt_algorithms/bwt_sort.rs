//! The main bwt_sort algorithm for the Rust version of the standard BZIP2 library.
//!
//! The main sorting algorithm is currently based on the standard Rust sort_unstable algorithm. When data is
//! larger than 5k bytes, we use a multi-threaded approach based on Rayon's par_sort_unstable algorithm.
//! 
//! Since different sorting algorithms are better suited for different kinds of data, this module contains a test
//! to determine whether the data would be better suited to the main algorithm or the fallback algorithm.
//! 
//! 
use super::sais_fallback::sais_entry;
use crate::bwt_algorithms::sais_fallback::lms_complexity;
use log::info;
use rayon::prelude::*;
/*
I tried a varient that used a double length block to avoid the nested equality checks
in block_compare, but it was barely faster.
*/

/// Encode data using the Burrows-Wheeler-Transform. Requires a u8 slice of data to be sorted. 
/// This returns a u32 key and a u8 vec of the BWT data.
pub fn bwt_encode(rle1_data: &[u8]) -> (u32, Vec<u8>) {
    // Test data longer than 5k bytes to help select the best algorithm
    if rle1_data.len() > 5_000 && lms_complexity(&rle1_data[0..5_000.min(rle1_data.len())]) < 0.35 {
        info!("Using SA-IS algorithm.");
        return sais_entry(rle1_data);
    }
    
    info!("Using native algorithm.");
    // Create index into block. Index is u32, which should be more than enough
    let mut index = (0_u32..rle1_data.len() as u32).collect::<Vec<u32>>();

    // Sort index
    if rle1_data.len() > 40000 {
        index[..].par_sort_unstable_by(|a, b| block_compare(*a as usize, *b as usize, rle1_data));
    } else {
        index[..].sort_unstable_by(|a, b| block_compare(*a as usize, *b as usize, rle1_data));
    }
    // Get key and BWT output 
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

/// Decode a Burrows-Wheeler-Transform. Requires a key, a u8 slice containing the BWT data, and an array of the u8 frequencies
/// found in the data. Returns the decoded data as a u8 vec.
pub fn bwt_decode(key: u32, bwt_in: &[u8], freq_in: &[u32]) -> Vec<u8> {
    /*
    I have tried refactoring to reduce cache misses. To date, all variations seem to have excessive cache misses. 
    */

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
