use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Condvar, Mutex};

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
    // let mut f_out = File::create(fname).expect("Can't create .bz2 file");

    /*
    This works and is faster than single-threaded versions. Unfortunately it can use a lot more memory.

    This works by compressing each block in parallel. Depending on the sequence of when those blocks finish,
    this will hold compressed blocks in memory until it is their turn to be written.
    */

    // Initialize locking / waiting variables for multi-core synchonization
    let sync = Arc::new((Condvar::new(), Mutex::new(0)));
    // Initialize thread channel communication. Sends block data, sequence number, and indicator whether
    //  this is the last block
    let (tx, rx) = std::sync::mpsc::channel();
    // Initialize a bitwriter.
    let mut bw = BitWriter::new(fname, opts.block_size, rx);

    // Build the RLE1 blocks and compress them
    rle1_blocks
        .into_iter()
        .enumerate()
        .par_bridge()
        .for_each(|(i, (crc, block, last_block))| {
            tx.send((compress_block(&block, crc), i, last_block));
        });
    Ok(())
}
