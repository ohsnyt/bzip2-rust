use rayon::prelude::*;

/// Returns a frequency count of the input data. Uses parallelism when data set is over 90k.
pub fn freqs(data: &[u8]) -> Vec<u32> {
    const FREQXOVER: usize = 90000;
    //Create the freq vec using a parallel approach
    if data.len() >= FREQXOVER {
        //Create the freq vec using a parallel approach
        data.par_iter()
            .fold(
                || vec![0_u32; 256],
                |mut freqs, &el| {
                    freqs[el as usize] += 1;
                    freqs
                },
            )
            .reduce(
                || vec![0_u32; 256],
                |s, f| s.iter().zip(&f).map(|(a, b)| a + b).collect::<Vec<u32>>(),
            )
    } else {
        data.iter().fold(vec![0_u32; 256], |mut freqs, &el| {
            freqs[el as usize] += 1;
            freqs
        })
    }
}
