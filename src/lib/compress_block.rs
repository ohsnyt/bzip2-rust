use super::{
    bwt::bwt_encode, compress::Block, mtf::mtf_encode, rle1::rle1_encode, rle2::rle2_encode, huffman::huf_encode,
};
use std::io::Error;


/// Compress one block and write out the stream.
/// Handles stream header and footer also.
pub fn compress_block(block: &Block) -> Result<(), Error > {
    // If this is the first block, write the stream header
    if block.seq == 1 {
        // Put the header onto the bit stream
        block.bw.out8('B' as u8);
        block.bw.out8('Z' as u8);
        block.bw.out8('h' as u8);
        block.bw.out8(block.block_size + 0x30);
    }

    // Next write the block header: Six bytes of magic,
    //   4 bytes of crc data, 1 bit for Randomized flag.
    block.bw.out24(0x18_314159); // magic bits  1-24
    block.bw.out24(0x18_265359); // magic bits 25-48
    block.bw.out32(block.block_crc); // crc
    block.bw.out24(0x01_000000); // One zero bit

    // Before we can write the key, we need to do the BWT
    let data = rle1_encode(&block.data);
    let (key, mut data) = bwt_encode(&data);

    // Now that we have the key, we can write the 24bit BWT key
    block.bw.out24(0x18_000000 | key); // and 24 bit key

    // Now send the BTW data off for the MTF transform...
    //  MTF also returns the symbol map that we need for decompression.
    let (mdata, symbol_map) = mtf_encode(&data);

    // We don't need data any more.
    drop(data);

    // ...followed by the RLE2 transform. These two may later be combined.
    let rle2_data = rle2_encode(&mdata);

    // We don't need mdata any more.
    drop(mdata);

    // Now for the compression - the Huffman encoding
    let result = huf_encode(&rle2_data, &mut block.bw, symbol_map);

    block.bw.out32(block.block_crc);
    
    if block.is_last {
        block.bw.out32(block.stream_crc);
        block.bw.flush();
    }
    result
}
