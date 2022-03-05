/*
BZip2 RLE2 step happens after the BW transform and after the MTF transform. The RLE2 format,
incorporates RUNA, RUNB and EOB (End Of Block) symbols into the Huffman symbol list.
It transforms ONLY the runs of zeros produced by the MTF transform.

A single 0 is converted into RUNA. Two 0s are converted to run B. Three 0s = RUNA RUNA, etc.

To illustrate (using A and B for RUNA and RUNB)
Input from MTF:  [1042300000142003450000000021]
Output from RLE2:[1A423AB   142B 345BAA     21]
(The other characters are bits as well, but we are illustrating what happens to the 0s.)

Within the Bzip2 Huffman symbol scheme, RUNA is encoded as a 0 bit, RUNB is encoded as
a 1 bit, the other symbols from the input are encoded in the standard huffman way, and EOB
is the last symbol in the Huffman symbol list.

Since we could have up to 258 symbols now, we will shift from u8 to u16 encoding.
*/

/// Symbols used in MTF and Huffman algorithms
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum MTF {
    /// Used with RUNB to encode runs of zeros
    RUNA,
    /// Used with RUNA to encode runs of zeros
    RUNB,
    /// u8 symbols from 1-255
    Sym(u8),
    /// Unique End Of Block symbol
    EOB,
}

/// Does RLE only on character 0 u8. Data is encoded in a unique Bzip2 way.
pub fn rle2_encode(v: &[u8]) -> Vec<MTF> {
    let mut zeros: u32 = 0;
    let cap = v.len() + 2;
    let mut out: Vec<MTF> = Vec::with_capacity(cap);
    //let mut biggest: u32 = 0;
    for i in 0..v.len() {
        if v[i] == 0 {
            zeros += 1;
        } else {
            if zeros > 0 {
                out.append(&mut rle2_encode_runs(zeros));
                zeros = 0;
            }
            out.push(MTF::Sym(v[i] + 1));
            //if v[i] as u32 + 2 > biggest {biggest = v[i] as u32 + 2};
        }
    }
    out.append(&mut rle2_encode_runs(zeros));
    out.push(MTF::EOB);
    //println!("REL2:{:?}", out);
    out
}

/// Unique encoding for any run of 0_u8
fn rle2_encode_runs(r: u32) -> Vec<MTF> {
    if r == 0 {
        return vec![];
    }
    let mut run = r - 1;
    let mut out: Vec<MTF> = vec![];
    loop {
        //run >>= 1;
        match run & 1 {
            1 => out.push(MTF::RUNB),
            _ => out.push(MTF::RUNA),
        }
        if run < 2 {
            break;
        };
        run = (run - 2) / 2;
    }
    out
}

pub fn rle2_decode(v: Vec<&MTF>) -> Vec<u8> {
    let mut zeros = 0;
    let mut bit_multiplier = 1;
    let mut out: Vec<u8> = Vec::with_capacity(v.len()*3/2);   // assume about 50% expansion space

    for mtf in v {
        if zeros > 2*1024*1024 {std::process::exit(100)} // Got a problem, pinkie.
        match mtf {
            MTF::RUNA => {
                zeros += 1 * bit_multiplier;
                bit_multiplier *= 2;
            }
            MTF::RUNB => {
                zeros += 2 * bit_multiplier;
                bit_multiplier *= 2;
            }
            MTF::Sym(n) => {
                if zeros > 0 {
                    while zeros / 1000 > 0 {
                        out.extend(vec![0; 1000]);
                        zeros -= 1000;
                    }
                    out.extend(vec![0; zeros]);
                    bit_multiplier = 1;
                    zeros = 0;
                }
                out.push(*n)
            }
            MTF::EOB => break,
        }
    }
    if zeros > 0 {
        out.extend(vec![0; zeros as usize]);
    };
    out
}
