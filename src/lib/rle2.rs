/*
BZip2 RLE2 step happens after the BW transform and after the u16 transform. The RLE2 format,
incorporates RUNA, RUNB and EOB (End Of Block) symbols into the Huffman symbol list.
It transforms ONLY the runs of zeros produced by the u16 transform.

A single 0 is converted into RUNA. Two 0s are converted to run B. Three 0s = RUNA RUNA, etc.

To illustrate (using A and B for RUNA and RUNB)
Input from u16:  [1042300000142003450000000021]
Output from RLE2:[1A423AB   142B 345BAA     21]
(The other characters are bits as well, but we are illustrating what happens to the 0s.)

Within the Bzip2 Huffman symbol scheme, RUNA is encoded as a 0 bit, RUNB is encoded as
a 1 bit, the other symbols from the input are encoded in the standard huffman way, and EOB
is the last symbol in the Huffman symbol list.

Since we could have up to 258 symbols now, we will shift from u8 to u16 encoding.
(Earlier implementations in Rust used Enums with values for RUNA, RUNB, EOB and Sym(u8).
However it proved to require lots of lookups through BtreeMaps. I have moved back to an array of u16s in
hopes of signficant speed advantages of direct indexing instead of searches.)
*/

use log::error;

use super::compress::Block;

pub(crate) const RUNA: u16 = 0;
pub(crate) const RUNB: u16 = 1;

/// Does run-length-encoding only on byte 0_u8. Output (a Vec<u16>) is encoded in a unique Bzip2 way 
/// and placed in temp_vec within Block
pub fn rle2_encode(block: &mut Block) {
    let mut zeros: u32 = 0;
    // we will need an End Of Block byte to be one larger than the last byte recorded.
    let mut eob = 0;
    // we will need a frequency count of all symbols
    let mut freq_out = [0u32; 258];

    // iterate through the input, turning runs of 1+ zeros into RUNA/RUNB sequences
    // shift all other indexes up one
    // count frequencies
    for &el in block.data.iter() {
        // if we find a zero
        if el == 0 {
            // increment the counter, even if we only find one of them
            zeros += 1;
            // and go look for more. Otherwise...
        } else {
            // We didn't find a zero. So If we have any pending zeros to put out
            if zeros > 0 {
                // write out the pending zeros using the special bzip2 coding
                block.temp_vec.extend(rle2_encode_runs(zeros, &mut freq_out).iter());
                // and reset the zeros counter
                zeros = 0;
            }
            // All non-zeros are incremented by 1 to get past the RUNA/RUNB sequences
            // This requires us to move from u8 to u16 (at least)
            let tmp = el as u16 + 1;
            //Write out the pending character with the value incremented by 1
            block.temp_vec.push(tmp);
            // Increment the frequency counts
            freq_out[tmp as usize] += 1;
            // Alway look for the largest value so we can mark the eob as +1
            block.eob = block.eob.max(tmp);
        }
    }
    block.temp_vec.extend(rle2_encode_runs(zeros, &mut freq_out).iter());
    // Increment the eob symbol to be one more than the largest symbol we found.
    block.eob += 1;
    // Write out the EOB to the stream.
    block.temp_vec.push(block.eob);
}

/// Unique encoding for any run of 0_u8
fn rle2_encode_runs(r: u32, counts: &mut [u32]) -> Vec<u16> {
    // if the run is 0, return empty vec
    if r == 0 {
        return vec![];
    }
    // otherwise, reduce the run count by 1
    let mut run = r - 1;
    // prepare the return vec
    let mut out: Vec<u16> = vec![];
    // while the last bit > 1, push the last bit, increment the count, decrement run
    loop {
        let bit = (run & 1) as u16;
        out.push(bit);
        counts[bit as usize] += 1;
        if run < 2 {
            break;
        }
        run = (run - 2) / 2;
    }
    // and return the unique bzip2 run of RUNA/RUNB (bit 0, bit 1)
    out
}

/// Does run-length-decoding from rle2_encode.
pub fn rle2_decode(v: &[u16]) -> Vec<u8> {
    // Initialize counters
    let mut zeros: i32 = 0;
    let mut bit_multiplier = 1;
    // create a vec with 50% more capacity than the input. This is a guestimate.
    let mut out: Vec<u8> = Vec::with_capacity(v.len() * 3 / 2);

    // iterate through the input, doing the conversion as we find RUNA/RUNB sequences
    for mtf in v {
        // Blow up if the run is too big - this should be more elegant in the future
        if zeros > 2 * 1024 * 1024 {
            error!("Run of zeros exceeded a million - probably input bomb.");
            std::process::exit(100)
        }
        match *mtf {
            // If we found RUNA, do magic to calculate how many zeros we need
            RUNA => {
                zeros += bit_multiplier;
                bit_multiplier *= 2;
            }
            // If we found RUNB, do magic to calculate how many zeros we need
            RUNB => {
                zeros += 2 * bit_multiplier;
                bit_multiplier *= 2;
            }
            // Anything else, output any pending run of zeros
            n => {
                if zeros > 0 {
                    while zeros / 1000 > 0 {
                        out.extend(vec![0; 1000]);
                        zeros -= 1000;
                    }
                    out.extend(vec![0; zeros as usize]);
                    bit_multiplier = 1;
                    zeros = 0;
                }
                // and then output the symbol, decremented down by one (since RUNA/RUNB is gone)
                out.push((n - 1) as u8);
            }
        }
    }
    // If we didn't find a non-zero before the end of the data, write any pending zeros
    if zeros > 0 {
        out.extend(vec![0; zeros as usize]);
    };
    // Remove the EOB from the stream that we added during RLE2 encode.
    let _ = out.pop();
    // return the expanded input
    out
}

#[test]
fn rle2_run_encode_55() {
    let mut counts = vec![0, 0];
    assert_eq!(rle2_encode_runs(55, &mut counts), [0, 0, 0, 1, 1])
}

#[test]
fn rle2_run_encode_8() {
    let mut counts = vec![0, 0];
    assert_eq!(rle2_encode_runs(8, &mut counts), [1, 0, 0])
}

#[test]
fn rle2_run_encode_5() {
    let mut counts = vec![0, 0];
    assert_eq!(rle2_encode_runs(5, &mut counts), [0, 1])
}

// For decode tests, we have to reduce the output by one byte because the
// decoder removes the last byte (assuming it is an eof symbol).
#[test]
fn rle2_decode_zero_run_5() {
    assert_eq!(
        rle2_decode(&[0, 1]),
        vec![0; 5 - 1 as usize]
    )
}

#[test]
fn rle2_zero_run_roundtrip_a() {
    let n = 473;
    let mut counts = vec![0, 0];
    assert_eq!(
        rle2_decode(&rle2_encode_runs(n, &mut counts)),
        vec![0; n as usize - 1]
    )
}

#[test]
fn rle2_zero_run_roundtrip_b() {
    let n = 472;
    let mut counts = vec![0, 0];
    assert_eq!(
        rle2_decode(&rle2_encode_runs(n, &mut counts)),
        vec![0; n as usize - 1]
    )
}
