const SCRATCH: usize = 12_usize;
const BWT_NUM_SEGMENTS: usize = 8;

struct Ibwtdata2 {
    syms: usize,
    index: usize,
}

pub fn bwt_decode_2x1(key: u32, bwt_in: &[u8]) -> Vec<u8> {
    let bwt_size = bwt_in.len() + 1;

    //Ibwtdata2 = bwt_size + SCRATCH;
    //COMPILER_ASSERT( IBWT_SCRATCH_PER_BYTE >= 12 );

    // NOTE: Here the algorithm is expecting 8 keys so it can do 8 interleaved decodings
    // let mut key = keys[0];

    let mut key = key as usize;

    // Get a freq count of symbols
    let mut freq = [0_usize; 256];
    for i in 0..bwt_in.len() {
        freq[bwt_in[i] as usize] += 1;
    }
    // Then transform it into a cumulative count of frequency counts
    let mut sum = 0_usize;
    freq.iter_mut().map(|mut freq| {
        *freq += sum;
        sum += *freq
    });

    //Build a last-first vector to find the previous character in the original data
    let mut lf = vec![0; bwt_in.len()];
    for (i, &s) in bwt_in.iter().enumerate() {
        lf[i] = freq[s as usize];
        freq[s as usize] += 1
    }

    // build LF2 :
    let mut ibwtdata2 = vec![];

    for i in 0..bwt_in.len() {
        let lf1 = lf[i];
        let s1 = bwt_in[i];
        let lf2 = lf[lf1];
        // s2 is the symbol that precedes s1. It is not the symbol at F[]
        // which could be found from the cumprob table.	(That's the successor to s2.)
        let s2 = bwt_in[lf1];

        ibwtdata2.push(Ibwtdata2 {syms: s1 as usize | ((s2 as usize) << 8) , index: lf2});
    }

    /*----------------------------------------------------------------
    THE BELOW RUNS EIGHT STREAMS OF DECODING
    let segment_len = bwt.len()/BWT_NUM_SEGMENTS;
    segment_len &= !0xF;

    let src0 = 0;
    let src1 = keys[1];
    let src2 = keys[2];
    let src3 = keys[3];
    let src4 = keys[4];
    let src5 = keys[5];
    let src6 = keys[6];
    let src7 = keys[7];

    for outi in 0..segment_len {
        //skip every other outi
        if outi % 2 == 1 {
            continue
        } else {
        src0 = ibwtdata2[src0].index;
        src1 = ibwtdata2[src1].index;
        src2 = ibwtdata2[src2].index;
        src3 = ibwtdata2[src3].index;
        src4 = ibwtdata2[src4].index;
        src5 = ibwtdata2[src5].index;
        src6 = ibwtdata2[src6].index;
        src7 = ibwtdata2[src7].index;
    }

    uint32_t src = src7;
    for(intptr_t outi = segment_len*BWT_NUM_SEGMENTS; outi < raw_size;outi++)
    {
        raw[outi] = (uint8_t) bwt[src];
        src = LF[src];
    }
    ---------------------------------------------------------------- */
    let mut raw = vec![0u8; bwt_in.len()];
    let mut src = key;
    for outi in 0..bwt_in.len() {
        raw[outi] = bwt_in[src];
        src = lf[src];
    }
    raw
    //ASSERT( src == (uint32_t)key );
}
