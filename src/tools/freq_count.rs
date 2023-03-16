use rayon::prelude::*;

/// Returns a frequency count of the input data. Uses parallelism when data set is over 64k.
pub fn freqs(data: &[u8]) -> Vec<u32> {
    if data.len() > 64_000 {
        // 16k is pretty much the sweet spot for chunk size.
        data.par_chunks(16_000)
            .fold(
                || vec![0_u32; 256],
                |mut freqs, chunk| {
                    chunk.iter().for_each(|&el| freqs[el as usize] += 1);
                    freqs
                },
            )
            .reduce(
                || vec![0_u32; 256],
                |s, f| s.iter().zip(&f).map(|(a, b)| a + b).collect::<Vec<u32>>(),
            )
    } else {
        let mut freqs = vec![0_u32; 256];
        data.iter().for_each(|&el| freqs[el as usize] += 1);
        freqs
    }
}
