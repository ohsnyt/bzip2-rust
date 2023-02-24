use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::prelude::MetadataExt;

use log::{debug, info};

use crate::bitstream::bitwriter::BitWriter;
use crate::tools::crc::{do_crc, do_stream_crc};

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
    let mut block_reader = RLE1Block::new(source_file, block_size);

    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.files[0].clone();
    fname.push_str(".bz2");
    let mut f_out = File::create(fname).expect("Can't create .bz2 file");

    //----- Prepare to loop through blocks of data and process it.   PROBABLY DON'T NEED EITHER OF THESE
    let mut bytes_left = fin_metadata.size() as usize;
    let mut sequence = 1_usize;

    // HOW DOES RAYON WORK WITH DOLIN THOSE OUT AND GETTIN THEM BACK FOR THE NEXT STEP?
    // I THINK EACH SHOULD BUILD A BITWRITER AND RETURN THE COMPRESSED HUFFMAN VEC, WHICH WE THEN
    // WRITE OUT IN SEQUENCE HERE.
    let mut stream_crc = 0;
    block_reader
    .iter()
    .for_each(|(crc, block)| {
        stream_crc = do_stream_crc(stream_crc, crc);
        compress_block(&block, crc , stream_crc, block_size, sequence, block.len() < block_size);
        sequence +=1;
        }).collect();

        // Update and record the stream crc
        block.stream_crc = do_stream_crc(block.stream_crc, block.block_crc);
        debug!(
            "Block crc is {}, stream crc is {}",
            block.block_crc, block.stream_crc,
        );
        timer.mark("crcs");
        // Do the compression, allowing choice between sorting algorithms for the BWTransform
        if opts.algorithm.is_none() {
            compress_block(
                &mut bw,
                &mut block,
                opts.block_size,
                Algorithms::Julian,
                opts.iterations,
            );
        } else {
            compress_block(
                &mut bw,
                &mut block,
                opts.block_size,
                opts.algorithm.as_ref().to_owned().unwrap().clone(),
                opts.iterations,
            );
        }

        info!(
            "Wrote block. Bitstream length is {} bytes. CRC is {}.\n",
            &bw.output.len(),
            &block.block_crc
        );

        // Write out the data in the bitstream buffer.
        f_out.write_all(&bw.output)?;

        // Clear the bitstream buffer since we wrote out the data from this block
        bw.output.clear();
        block.block_crc = 0;
    

    Ok(())
}


