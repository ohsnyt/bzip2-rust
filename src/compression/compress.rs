use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::prelude::MetadataExt;

use log::{debug, info};

use crate::bitstream::bitwriter::BitWriter;

use super::compress_block::compress_block;
use crate::tools::cli::{Algorithms, BzOpts};
use crate::tools::rle1::RLE1Block;

use rayon::prelude::*;

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
    let mut source_file = File::open(&fname)?;
    let fin_metadata = fs::metadata(&fname)?;

    // Initialize the RLE1 reader/iterator. This reads the input file and creates blocks of the
    // proper size to then be compressed.
    let block_size = (opts.block_size as usize * 100000) - 19;
    let mut rle1_blocks = RLE1Block::new(source_file, block_size);

    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.files[0].clone();
    fname.push_str(".bz2");
    let mut f_out = File::create(fname).expect("Can't create .bz2 file");

    //----- Prepare to loop through blocks of data and process it.   PROBABLY DON'T NEED EITHER OF THESE
    let mut bytes_left = fin_metadata.size() as usize;
    let mut sequence = 1_usize;

    // HOW DOES RAYON WORK WITH DOLING THOSE OUT AND GETTIN THEM BACK FOR THE NEXT STEP?
    // I THINK EACH SHOULD BUILD A BITWRITER AND RETURN THE COMPRESSED HUFFMAN VEC, WHICH WE THEN
    // WRITE OUT IN SEQUENCE HERE.

    // NOTE: par_bridge is not supposed to be very efficient. Look into this.

    // I SHOULD PROBABLY USE A STRUCT TO CARRY THE BLOCK INFO FORWARD - THE RLE1 DATA, THE BLOCK block_crc, STREAM block_crc,
    //   and for the first and last block: BLOCK SIZE, IS_LAST
    // BUT IF I'M WRITING HEADERS AND FOOTERS SEPARATELY, PERHAPS I CAN ADD THOSE ELSEWHERE.

    let huff_blocks = rle1_blocks
        .into_iter()
        .map(|(block_crc, stream_crc, block)| compress_block(&block, block_crc, stream_crc))
        .collect::<Vec<u8>>()
        ;

    // BLOCK SIZE IS PROBABLY WRONG.
    let bw = BitWriter::new(block_size);
    // First write file stream header onto the stream
    bw.out8(b'B');
    bw.out8(b'Z');
    bw.out8(b'h');
    bw.out8(block_size as u8 + 0x30);

    // left shift each huff_block so there isn't empty space at the end of each and write it.
    to_do();

    // At the last block, write the stream footer magic and  block_crc and flush the output buffer
    bw.out24(0x18_177245); // magic bits  1-24
    bw.out24(0x18_385090); // magic bits 25-48
    bw.out32(stream_crc as u32);
    bw.flush();

    // Write out the data in the bitstream buffer.
    f_out.write_all(&bw.output)?;

    Ok(())
}
