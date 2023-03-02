use std::fs::File;
use std::io::{self, Write};

use crate::bitstream::bitwriter::BitWriter;
use crate::tools::crc::do_stream_crc;

use super::compress_block::compress_block;
use crate::tools::cli::BzOpts;
use crate::tools::rle1::RLE1Block;

use rayon::prelude::*;
#[allow(clippy::unusual_byte_groupings)]
/*
    NOTE: I WILL EVENTUALLY CHANGE THIS SO IT WORKS WITH A C FFI.

    This is repsonsible for creating the bitstream writer, a struct that
    contains the block data passes to the block compression routine,
    and an indictor of whether we are done processing all the data.

    Each block will be passed to compress_block.rs for RLE1, BWTransform,
    MTF, RLE2, and huffman compression.

    Again, this will iterate multiple times to get through the input file.
*/

/// Compress the input file defined in opts <BzOpts>. Modified for multi-core processing.
pub fn compress(opts: &mut BzOpts) -> io::Result<()> {
    /*
      Since this can be parallel, we may need to pass a reference to the u8 data as well as a sequence number.
      We will receive back the compressed data and sequence number. We will then assemble the compressed
      data segments and write them out to the output file.

      THE ROUTINES FOR FILE I/O ARE RUDEMENTARY, AND DO NOT PROPERLY RESOLVE ALL I/O ERRORS.
    */

    // Prepare to read the data.
    let fname = opts.files[0].clone();
    let source_file = File::open(&fname)?;
    // let fin_metadata = fs::metadata(&fname)?;

    // Initialize the RLE1 reader/iterator. This reads the input file and creates blocks of the
    // proper size to then be compressed.
    let block_size = (opts.block_size as usize * 100000) - 19;
    let rle1_blocks = RLE1Block::new(source_file, block_size);

    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.files[0].clone();
    fname.push_str(".bz2");
    let mut f_out = File::create(fname).expect("Can't create .bz2 file");

    /*
    This works and is faster than single-threaded versions. Unfortunately it is a hog of memory.

    This works by compressing each block and holding those compressed blocks in memory until we have them all.
    We then write them out to the output stream.

    NOTE: par_bridge is not supposed to be very efficient. Look into this.
    */

    // Initialize the stream crc
    let mut stream_crc = 0;
    // Initialize a bitwriter.
    let mut bw = BitWriter::new(opts.block_size);

    // Build the RLE1 blocks and process
    let huff_blocks = rle1_blocks
        .into_iter()
        .par_bridge()
        .map(|(block_crc, block)| (block_crc, compress_block(&block, block_crc)))
        .collect::<Vec<(u32, (Vec<u8>, u8))>>();

    // First write file stream header onto the stream
    bw.out8(b'B');
    bw.out8(b'Z');
    bw.out8(b'h');
    bw.out8(opts.block_size as u8 + 0x30);

    // left shift each huff_block so there isn't empty space at the end of each and write it.
    huff_blocks.iter().for_each(|(crc, (block, last_bits))| {
        stream_crc = do_stream_crc(stream_crc, *crc);
        block
            .iter()
            .take(block.len() - 1)
            .for_each(|byte| bw.out8(*byte));
        // Unpack the last byte by right shifting it. If last_bits is zero, then there was no last
        // partial byte so write out the entire last byte.
        if last_bits == &0 {
            bw.out8(*block.last().unwrap())
        } else {
            bw.out24((*last_bits as u32) << 24 | *block.last().unwrap() as u32 >> (8 - *last_bits));
        }
    });

    // At the last block, write the stream footer magic and  block_crc and flush the output buffer
    bw.out24(0x18_177245); // magic bits  1-24
    bw.out24(0x18_385090); // magic bits 25-48
    bw.out32(stream_crc as u32);
    bw.flush();

    // Write out the data in the bitstream buffer.
    f_out.write_all(&bw.output)?;

    Ok(())
}
