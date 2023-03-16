use crate::bitstream::bitpacker::BitPacker;
use crate::tools::rle2_mtf::rle2_mtf_encode;
use crate::bwt_algorithms::bwt_sort::bwt_encode;
use log::{debug, trace};
use crate::huffman_coding::huffman::huf_encode;

#[allow(clippy::unusual_byte_groupings)]
/// Called by Compress, this handles one block and returns a vec of packed huffman data and the valid bit count of the last byte.
pub fn compress_block(block: &[u8], block_crc: u32) -> (Vec<u8>, u8) {
    // Initialize A bitwriter vec to the block size to avoid resizing. Block.len is a very generous size.
    let mut bp = BitPacker::new(block.len());

    // For each block, write the block header:
    // Six bytes of magic, 4 bytes of block_crc data, 1 bit for Randomized flag.
    trace!(
        "\r\x1b[43mWriting block magic and block_crc at {}.    \x1b[0m",
        bp.loc()
    );
    bp.out24(0x18_314159); // magic bits  1-24
    bp.out24(0x18_265359); // magic bits 25-48
    bp.out32(block_crc as u32); // block_crc
    trace!(
        "\r\x1b[43mWriting randomize bit at {}.    \x1b[0m",
        bp.loc()
    );
    bp.out24(0x01_000000); // One zero bit

    // Do BWT using the native algorithm with sais as fallback
    let (key, bwt_data) = bwt_encode(block);

    // Now that we have the key, we can write the 24bit BWT key
    trace!("\r\x1b[43mWriting key at {}.    \x1b[0m", bp.loc());
    bp.out24(0x18_000000 | key as u32); // and 24 bit key

    let (rle2, freq, symbol_map) = rle2_mtf_encode(&bwt_data);

    // Get the eob character
    let eob = rle2[rle2.len() - 1];

    // Now for the compression - the Huffman encoding (which also writes out data)
    huf_encode(&mut bp, &rle2, &freq, eob, &symbol_map);

    debug!(
        "\n         {} bytes in block, {} after MTF & RLE2 coding, {} syms in use",
        block.len(),
        rle2.len(),
        eob + 1,
    );
    // Flush the buffer before returning
    bp.flush();
    (bp.output, bp.padding)
}
