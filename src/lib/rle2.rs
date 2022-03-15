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

//use super::report::report;
pub(crate) const RUNA: u16 = 0;
pub(crate) const RUNB: u16 = 1;

/// Does RLE only on character 0 u8. Data is encoded in a unique Bzip2 way.
pub fn rle2_encode(v: &[u8]) -> (Vec<u16>, [u32; 258], u16) {
    let mut zeros: u32 = 0;
    // Rarely will rle2 expand the mtf data, so assume the same output size.
    let mut out: Vec<u16> = Vec::with_capacity(v.len());
    // we will need an End Of Block byte to be one larger than the last byte recorded.
    let mut eob = 0;
    // we will need a frequency count of all symbols
    let mut freq_out = [0u32; 258];

    // iterate through the input, turning runs of 1+ zeros into RUNA/RUNB sequences
    // shift all other indexes up one
    // count frequencies
    for &el in v.iter() {
        if el == 0 {
            zeros += 1;
        } else {
            if zeros > 0 {
                if zeros > 20 {
                    println! {"{}",zeros};
                };
                out.append(&mut rle2_encode_runs(zeros, &mut freq_out));
                zeros = 0;
            }
            let val = el as u16 + 1;
            out.push(val);
            freq_out[val as usize] += 1;
            eob = eob.max(val);
        }
    }
    out.append(&mut rle2_encode_runs(zeros, &mut freq_out));
    out.push(eob + 1);
    (out, freq_out, eob + 1)
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

    out
}

pub fn rle2_decode(v: &[u16]) -> Vec<u8> {
    let mut zeros: i32 = 0;
    let mut bit_multiplier = 1;
    let mut out: Vec<u8> = Vec::with_capacity(v.len() * 3 / 2); // assume about 50% expansion space
                                                                // let mut i = 0; //t  testing
    for mtf in v {
        if zeros > 2 * 1024 * 1024 {
            std::process::exit(100)
        } // Got a problem, pinkie.
        match *mtf {
            RUNA => {
                zeros += bit_multiplier;
                bit_multiplier *= 2;
            }
            RUNB => {
                zeros += 2 * bit_multiplier;
                bit_multiplier *= 2;
            }
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
                out.push(n as u8 - 1);
            }
        }
    }
    if zeros > 0 {
        out.extend(vec![0; zeros as usize]);
    };
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

#[test]
fn rle2_zero_run_roundtrip_a() {
    let n = 473;
    let mut counts = vec![0, 0];
    assert_eq!(
        rle2_decode(&rle2_encode_runs(n, &mut counts)),
        vec![0; n as usize]
    )
}

#[test]
fn rle2_zero_run_roundtrip_b() {
    let n = 472;
    let mut counts = vec![0, 0];
    assert_eq!(
        rle2_decode(&rle2_encode_runs(n, &mut counts)),
        vec![0; n as usize]
    )
}
