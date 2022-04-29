
///Burrows-Wheeler-Transform - based on https://github.com/aufdj
/// receives reference to incoming block of data and
/// returns key for final data decomcpression. Key is u32.
// pub fn bwt_encode(orig: &[u8]) -> (u32, Vec<u8>) {
//     // Create index into block. Index is u32, which should be more than enough
//     //let ext = orig.len();
//     //let mut index: Vec<(u8, usize)> = orig.iter().enumerate().map(|(i, &s)| (s, i)).collect();
//     //index.append(&mut (orig.iter().enumerate().map(|(i, &s)| (s, i + ext)).collect()));

//     let mut index = vec![0; orig.len()];
//     for i in 0..index.len() {
//         index[i as usize] = i as u32;
//     }
//     // Sort index
//     index[..].sort_by(|a, b| block_compare(*a as usize, *b as usize, orig));
//     // Try radix sort
//     //rdxsort::RdxSort::rdxsort(&mut index);
//     //info!("Known good index: {:?}", index);

//     // Get key and BWT output (assumes u32 is 4 bytes)
//     let mut key: u32 = 0;
//     let mut bwt = vec![0; orig.len()];
//     for i in 0..bwt.len() {
//         if index[i] == 0 {
//             key = i as u32;
//         }
//         if index[i] == 0 {
//             bwt[i] = orig[orig.len() - 1];
//         } else {
//             bwt[i] = orig[(index[i] as usize) - 1];
//         }
//     }
//     (key, bwt)
// }

// /// compare the next two chunks of the original data to decide which sorts first
// fn block_compare(a: usize, b: usize, block: &[u8]) -> std::cmp::Ordering {
//     let min = std::cmp::min(block[a..].len(), block[b..].len());

//     // Lexicographical comparison
//     let result = block[a..a + min].cmp(&block[b..b + min]);

//     // Implement wraparound if needed
//     if result == std::cmp::Ordering::Equal {
//         return [&block[a + min..], &block[0..a]]
//             .concat()
//             .cmp(&[&block[b + min..], &block[0..b]].concat());
//     }
//     result
// }
 
/// Decode a Burrows-Wheeler-Transform
pub fn bwt_decode(key: u32, btw_in: &[u8]) -> Vec<u8> {
    //first get a freq count of symbols
    let mut freq = [0; 256];
    for i in 0..btw_in.len() {
        freq[btw_in[i] as usize] += 1;
    }
    //then build a cumulative count of frequency counts (necessary??)
    let mut sum = 0;
    let mut sum_freq = [0; 256];
    for i in 0..256 {
        sum_freq[i] = sum;
        sum += freq[i];
    }
    //zero out the freq count of symbols to recount frequencies in the transformation vector
    let mut freq = [0; 256];
    //Build the transformation vector to find the next character in the original data
    let mut t_vec = vec![0; btw_in.len()];
    for (i, &s) in btw_in.iter().enumerate() {
        t_vec[freq[s as usize] + sum_freq[s as usize]] = i;
        freq[s as usize] += 1
    }
    // Transform the data
    let mut orig = Vec::new();
    let mut key = t_vec[key as usize];

    for _ in 0..btw_in.len() {
        orig.push(btw_in[key]);
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
