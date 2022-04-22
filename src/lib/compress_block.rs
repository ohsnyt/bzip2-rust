//use crate::lib::bwt_ds::bwt_encode;

use super::bwt::block_sort::block_sort;
use log::{info, trace};

use super::{
    bitwriter::BitWriter, compress::Block, huffman::huf_encode, mtf::mtf_encode, rle1::rle1_encode,
    rle2::rle2_encode,
};
#[allow(clippy::unusual_byte_groupings)]
/// Compress one block and write out the stream.
/// Handles stream header and footer also.
pub fn compress_block(data: &[u8], bw: &mut BitWriter, block: &Block, block_size: u8) {
    trace!("Starting compression at {}", bw.loc());
    // If this is the first block, write the stream header
    if block.seq == 1 {
        // Put the header onto the bit stream
        bw.out8(b'B');
        bw.out8(b'Z');
        bw.out8(b'h');
        bw.out8(block_size as u8 + 0x30);
    }

    trace!(
        "Block header starting at {} with crc of {}.",
        bw.loc(),
        block.block_crc
    );
    // Next write the block header: Six bytes of magic,
    //   4 bytes of crc data, 1 bit for Randomized flag.
    bw.out24(0x18_314159); // magic bits  1-24
    bw.out24(0x18_265359); // magic bits 25-48
    bw.out32(block.block_crc); // crc
    bw.out24(0x01_000000); // One zero bit

    // Before we can write the key, we need to do the BWT
    let rle_data = rle1_encode(data);

    // Remember the data length for reporting later
    let block_length = data.len();

    // Using SAIS algorithm
    //let bwt_combined = crate::lib::bwt_internal::bwt(rle_data);
    //let key = bwt_combined.end_of_string;
    //let bwt_data = bwt_combined.data;

    // compare to using my (slow) algorithm
    //let (key_ds, bwt_data_ds) = bwt_encode(&rle_data);
    //info!("Known good: {:?}", bwt_data_ds);

    // Using julians algorithm
    let (key, bwt_data) = block_sort(&rle_data, 30);
    /* if key_ds == key as u32 {
        info!("HURRAY! MATCHING KEYS.")
    } else {
        error!("Mismatched keys.")
    }
    if bwt_data_ds == bwt_data {
        info!("HURRAY!! MATCHING BWT DATA!\n")
    } else {
        error!("Mismatched data blocks.");
        //info!("good: {:?}", bwt_data_ds);
        //info!(" bad: {:?}", bwt_data);
    } */

    //trace!("Input is {:?}", std::str::from_utf8(data).unwrap());
    //trace!("BWT output is {:?}", std::str::from_utf8(&bwt_data).unwrap());
    trace!("Key is {}, at {} (three bytes long)", key, bw.loc());
    // Now that we have the key, we can write the 24bit BWT key
    bw.out24(0x18_000000 | key as u32); // and 24 bit key

    // Now send the BTW data off for the MTF transform...
    //  MTF also returns the symbol map that we need for decompression.
    let (mdata, symbol_map) = mtf_encode(&bwt_data);
    trace!("MTF output is {:?}", std::str::from_utf8(&mdata).unwrap());

    // We don't need bwt_data any more.
    //drop(bwt_data);

    // ...followed by the RLE2 transform. These two may later be combined.
    let (rle2_data, freq_out, eob) = rle2_encode(&mdata);
    // We don't need mdata any more.
    drop(mdata);

    trace!("Going to do huffman encoding at {}", bw.loc());
    // Now for the compression - the Huffman encoding (which also writes out data)
    let _result = huf_encode(bw, &rle2_data, &freq_out, symbol_map, eob);
    // SHOULD HANDLE RESULT ERROR

    // if this is the last block, write the stream footer magic and  crc and flush
    // the output buffer
    if block.is_last {
        trace!("Stream footer starting at {}", bw.loc());
        bw.out24(0x18_177245); // magic bits  1-24
        bw.out24(0x18_385090); // magic bits 25-48
        trace!("Final crc of {} starting at {}", block.stream_crc, bw.loc());
        bw.out32(block.stream_crc);
        bw.flush();
    }

    info!(
        "{} bytes in block, {} after MTF & RLE2 coding, {} syms in use",
        block_length,
        &rle2_data.len(),
        eob + 1,
    );
}
