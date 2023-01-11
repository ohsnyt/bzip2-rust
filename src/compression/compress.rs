use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::prelude::MetadataExt;

use log::{debug, info};

use crate::bitstream::bitwriter::BitWriter;
use crate::tools::crc::{do_crc, do_stream_crc};
use crate::Timer;

use super::compress_block::compress_block;
use crate::tools::cli::BzOpts;
use crate::tools::rle1::rle_encode;

/*
    NOTE: I WILL EVENTUALLY CHANGE THIS SO IT WORKS WITH A C FFI.

    This is repsonsible for creating the bitstream writer, a struct that
    contains the block data passes to the block compression routine,
    and an indictor of whether we are done processing all the data.

    Each block will be passed to compress_block.rs for RLE1, BWTransform,
    MTF, RLE2, and huffman compression.

    Again, this will iterate multiple times to get through the input file.
*/

pub struct Block {
    // Add in block data, sym_map, index, temp vec for data work???
    pub data: Vec<u8>,
    pub rle2: Vec<u16>,
    pub end: u32,
    pub key: u32,
    pub freqs: [u32; 258],
    pub sym_map: Vec<u16>,
    pub eob: u16,
    pub seq: u32,
    pub block_crc: u32,
    pub stream_crc: u32,
    pub budget: i32,
    pub is_last: bool,
}

/// Compress the input file defined in opts <BzOpts>. Requires a Timer.
pub fn compress(opts: &mut BzOpts, timer: &mut Timer) -> io::Result<()> {
    // Create the struct to pass data to compress_block.rs
    // Initialize the size of the data vec to the block size to avoid resizing
    let mut bw = BitWriter::new(opts.block_size as usize * 100000);

    // Initialize the block struct used by every block. Julian took 19 off the block.
    let mut block = Block {
        data: Vec::with_capacity(opts.block_size as usize * 100000 - 19),
        rle2: Vec::with_capacity(opts.block_size as usize * 100000 - 19),
        end: opts.block_size as u32 * 100000 - 19,
        key: 0,
        freqs: [0; 258],
        sym_map: Vec::with_capacity(17),
        eob: 0,
        seq: 0,
        block_crc: 0,
        stream_crc: 0,
        budget: 30,
        is_last: false,
    };

    // THE ROUTINES BELOW FOR FILE I/O ARE RUDEMENTARY, AND DO NOT PROPERLY RESOLVE
    // FILE METADATA AND ALL I/O ERRORS.
    // NOTE: All writes append with out deleting existing files!!

    // Initialize stuff to read the file
    //let input = data_in::init(opts)?;

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

    timer.mark("setup");

    //----- Prepare to loop through blocks of data and process it.
    //let mut bytes_processed = 0;
    let mut bytes_left = fin_metadata.size() as usize;
    // We need a read buffer that exists throughout the process
    let mut buf = vec![];

    while bytes_left > 0 {
        block.data.clear();
        block.rle2.clear();

        // Calculate how much data we need for this next block.
        //   We can't exceed the input file size, though.
        let bytes_desired = bytes_left.min(block.end as usize) as u32;
        let mut block_full = false;
        let mut processed = 0;

        // Get data and do the RLE1. We may need more than one read to fill the buffer
        while !block_full {
            if buf.is_empty() {
                //   Read 20% more than we need, if we have enough data left.
                buf = vec![0; (block.end as usize * 5 / 4).min(bytes_left)];
                fin.read_exact(&mut buf)
                    .expect("Could not read enough bytes.");
                processed = 0;
            }

            // Do the rle on a glob of data - hopefully more than we need
            let (bfull, used, new_data) =
                rle_encode(&buf, bytes_desired - processed - block.data.len() as u32);
            block_full = bfull;
            processed += used;

            // // Subtract what we got from what we wanted, safely (must be done before append!)
            // bytes_desired = bytes_desired.saturating_sub(new_data.len() as u32);

            // Add the data to block
            block.data.extend(new_data.iter());
            timer.mark("rle1");
            // Update the block end.
            block.end = block.data.len() as u32;

            if processed > buf.len() as u32 {
                println!("Pause here...")
            }
            // Do CRC on what we got each time
            block.block_crc = do_crc(block.block_crc, &buf[0..used as usize]);
            timer.mark("crcs");

            // Drain what we used from the buffer
            buf.drain(0..processed as usize);
            bytes_left -= processed as usize;
            if bytes_left == 0 {
                block.is_last = true;
                break;
            }
        }
        // Done with RLE1
        timer.mark("rle1");

        // We reached the block size we wanted, so process this block
        // Update the block sequence counter and inform the user
        block.seq += 1;
        info!("Starting block {}", &block.seq);
        timer.mark("setup");

        // Update and record the stream crc
        block.stream_crc = do_stream_crc(block.stream_crc, block.block_crc);
        debug!(
            "Block crc is {}, stream crc is {}",
            block.block_crc, block.stream_crc,
        );
        timer.mark("crcs");
        // Do the compression, allowing choice between sorting algorithms for the BWTransform
        compress_block(
            &mut bw,
            &mut block,
            opts.block_size,
            opts.algorithm.clone(),
            opts.iterations,
            timer,
        );

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

        timer.mark("bwt");
    }

    Ok(())
}
