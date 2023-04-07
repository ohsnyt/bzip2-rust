//! Optimized byte frequency counting for a slice of u8 data.
//!
//! Create an array of 256 u32 integers which hold the frequency counts of each byte found in the block of 
//! data given to the freqs function. 
//! 
//! NOTE: This will use multi-threading when the data is over 16k in length.
//!

use rayon::prelude::*;

/// Returns a frequency count of the input data. 
pub fn freqs(data: &[u8]) -> [u32;256] {
    if data.len() > 64_000 {
        // 16k is pretty much the sweet spot for chunk size.
        data.par_chunks(16_000)
            .fold(
                || [0_u32; 256],
                |mut freqs:[u32;256], chunk| {
                    chunk.iter().for_each(|&el| freqs[el as usize] += 1);
                    freqs
                },
            )
            .reduce(
                || [0_u32; 256],
                |s, f| {let x = s.iter().zip(&f).map(|(a, b)| a + b); let mut z = [0_u32; 256]; for (i, el) in x.enumerate() {z[i] = el}; z})
    } else {
        let mut freqs = [0_u32; 256];
        data.iter().for_each(|&el| freqs[el as usize] += 1);
        freqs
    }
}
