use std::fs::{self, File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, Write};
use std::os::unix::prelude::MetadataExt;

use log::{info, warn};

use crate::lib::crc::{do_crc, do_stream_crc};

use super::bitwriter::BitWriter;
use super::compress_block::compress_block;
use super::options::Status;
use super::rle1::rle_encode;
use super::{data_in, options::BzOpts};

/*
    NOTE: I AM IN THE PROGRESS OF CHANGING THIS SO IT WORKS WITH A C FFI.

    This is repsonsible for creating the bitstream writer, a struct that
    contains the block data passes to the block compression routine,
    and an indictor of whether we are done processing all the data.

    Each block will be passed to compress_block.rs for RLE1, BWTransform,
    MTF, RLE2, and huffman compression.

    Again, this will iterate multiple times to get through the input file.
*/

pub struct Block {
    //pub data: &'a [u8],
    pub block_size: usize,
    pub seq: u32,
    pub block_crc: u32,
    pub stream_crc: u32,
    pub is_last: bool,
}

/// Compress the input file defined in the command line.
pub fn compress(opts: &mut BzOpts) -> io::Result<()> {
    // Create the struct to pass data to compress_block.rs
    // Initialize the size of the data vec to the block size to avoid resizing
    let mut bw = BitWriter::new(opts.block_size as usize * 100000);

    /* Julian took 19 off the block size.
     */
    let mut block = Block {
        block_size: opts.block_size as usize * 100000 - 19,
        seq: 0,
        block_crc: 0,
        stream_crc: 0,
        is_last: false,
    };

    // THE ROUTINES BELOW FOR FILE I/O ARE RUDEMENTARY, AND DO NOT PROPERLY RESOLVE
    // FILE METADATA AND ALL I/O ERRORS.
    // NOTE: All writes append with out deleting existing files!!

    // Initialize stuff to read the file
    let input =  data_in::init(opts)?;

    // Prepare to read the data.
    let fname = opts.file.as_ref().unwrap().clone();
    let mut fin = File::open(&fname)?;
    let fin_metadata = fs::metadata(&fname)?;

    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.file.as_ref().unwrap().clone();
    fname.push_str(".bz2");
    let mut f_out = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&fname)?;

    //----- Prepare to loop through blocks of data and process it.
    let mut bytes_processed = 0;
    let mut bytes_left = fin_metadata.size() as usize;
    // We need a read buffer that exists throughout the process
    let mut buf = vec![];

    while bytes_left > 0 {
        // Calculate how much data we need for this next block.
        //   We can't exceed the input file size, though.
        let mut bytes_desired = block.block_size;
        // Create an empty vec for the next block (reduced if not much data available).
        let mut block_data = Vec::with_capacity((bytes_desired + 19).min(bytes_left*5/4));

        // Get data and do the RLE. We may need more than one read
        while bytes_desired > 0 && bytes_left > 0 {
            if buf.is_empty() {
                //   Read 20% more than we need, if we have enough data left.
                buf = vec![0; (block.block_size * 5 / 4).min(bytes_left)];
                fin.read_exact(&mut buf);
            }
            // Do the rle on a glob of data - hopefully more than we need
            let (processed, mut new_data) = rle_encode(&buf, bytes_desired);

            // Subtract what we got from what we wanted, safely (must be done before append!)
            bytes_desired = bytes_desired.saturating_sub(new_data.len());

            // Append the data into
            block_data.append(&mut new_data);
            // Do CRC on what we got
            block.block_crc = do_crc(block.block_crc, &buf[0..processed]);


            // Drain what we used
            buf.drain(0..processed);
            bytes_left -= processed;
            if bytes_left == 0 {
                block.is_last = true;
            }
        }
        // We reached the block size we wanted, so process this block
        // Update the block sequence counter and inform the user
        block.seq += 1;
        info!("Starting block {}", &block.seq);

        // Update and record the stream crc
        block.stream_crc = do_stream_crc(block.stream_crc, block.block_crc);
        warn!(
            "Block crc is {}, stream crc is {}",
            block.block_crc, block.stream_crc,
        );

        // Do the compression, allowing choice between sorting algorithms for the BWTransform
        compress_block(
            &block_data,
            &mut bw,
            &block,
            opts.block_size,
            &opts.algorithm,
        );

        // Write out what we have so we don't have to hold it all.
        f_out.write_all(&bw.output)?;
        info!(
            "Wrote block. Bitstream length is {} bytes. CRC is {}.\n",
            &bw.output.len(),
            &block.block_crc
        );

        // Clear the bitstream buffer since we wrote out the data from this block
        bw.output.clear();
        block.block_crc = 0;
    }
    Ok(())
}
