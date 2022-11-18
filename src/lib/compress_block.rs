//use crate::lib::bwt_ds::bwt_encode;

use super::bwt::{block_sort::block_sort, primary::main_sort::QsortData};
use log::info;

use super::{
    bitwriter::BitWriter, compress::Block, huffman::huf_encode, mtf::mtf_encode, rle2::rle2_encode,
};
#[allow(clippy::unusual_byte_groupings)]
/// Compress one block and write out the stream.
/// Handles stream header and footer also.
pub fn compress_block(
    bw: &mut BitWriter,
    block: &mut Block,
    block_size: u8,
    algorithm: &super::cli::Algorithms,
    qs: &mut QsortData,
    iterations: usize,
) {
    // Adjust qs fields
    qs.end = block.end as usize;
    qs.budget = block.budget;
    
    // If this is the first block, write the stream header
    if block.seq == 1 {
        // Put the header onto the bit stream
        bw.out8(b'B');
        bw.out8(b'Z');
        bw.out8(b'h');
        bw.out8(block_size as u8 + 0x30);
    }

    // For each block, write the block header:
    // Six bytes of magic, 4 bytes of crc data, 1 bit for Randomized flag.
    bw.out24(0x18_314159); // magic bits  1-24
    bw.out24(0x18_265359); // magic bits 25-48
    bw.out32(block.block_crc); // crc
    bw.out24(0x01_000000); // One zero bit

    match algorithm {
        // Using simple DS algorithm
        crate::lib::cli::Algorithms::Simple => {
            info!("Using DS simple algorithm.");
            let result = crate::lib::bwt_ds::bwt_encode(&block.data);
            (block.key, block.data) = result
        }
        // Using SAIS algorithm from ribzip2
        crate::lib::cli::Algorithms::SAIS => {
            info!("Using SAIS algorithm.");
            //block_sort(data, 30);
            let result = crate::lib::bwt_ribzip::bwt_internal::bwt(&block.data);
            (block.key, block.data) = result
        }
        /* // Using voracious_radix_sort and DS algorithm
        crate::lib::cli::Algorithms::Radix => {
            info!("Using DS radix algorithm.");
            crate::lib::bwt_ds_2::bwt_encode(data)
        } */
        // Using julians algorithm
        crate::lib::cli::Algorithms::Julian => {
            info!("Using Julians algorithm.");
            block_sort(block, qs)
        }
        _ => {
            info!("Default: Using Julians algorithm.");
            block_sort(block, qs)
        }
    };

    // Now that we have the key, we can write the 24bit BWT key
    bw.out24(0x18_000000 | block.key as u32); // and 24 bit key

    // Now send the BTW data off for the MTF transform...
    mtf_encode(block);

    // ...followed by the RLE2 transform. These two may later be combined.
    rle2_encode(block);

    // Now for the compression - the Huffman encoding (which also writes out data)
    let _result = huf_encode(bw, block, iterations);
    // SHOULD HANDLE RESULT ERROR

    // if this is the last block, write the stream footer magic and  crc and flush
    // the output buffer
    if block.is_last {
        bw.out24(0x18_177245); // magic bits  1-24
        bw.out24(0x18_385090); // magic bits 25-48
        bw.out32(block.stream_crc);
        bw.flush();
    }

    info!(
        "{} bytes in block, {} after MTF & RLE2 coding, {} syms in use",
        block.end,
        &block.temp_vec.len(),
        block.eob + 1,
    );
}
