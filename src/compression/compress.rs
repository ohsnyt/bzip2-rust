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
    // We need a read buffer that exists throughout the process. Initially it must be empty.
    let mut buf = Vec::with_capacity((block.end as usize).min(bytes_left));

    while bytes_left > 0 {
        // Make sure the block data vecs do not have old data
        block.data.clear();
        block.rle2.clear();

        // Set the maximum free space available in this block. Initally block.end is the max block size.
        let mut free_space = block.end as usize;

        // Count how much of the input buffer have we processed used so far
        let mut processed: usize = 0;

        // Get data and do the RLE1. We may need more than one read to fill the buffer
        while free_space > 0 {
            // If we don't have any data in the buffer, go read some more data
            if buf.is_empty() {
                // First allocate space for the read buffer
                buf = vec![0_u8; bytes_left.min(block.end as usize)];
                //   Read a whole block worth of data, or until the end of the input file.
                fin.read_exact(&mut buf)
                    .expect("Could not read enough bytes.");
                // New buffer read, so we haven't processed any of it yet
                processed = 0;
            }

            // Do the rle1 on a glob of data
            let (used, new_data) = rle_encode(&buf, free_space);
            processed += used as usize;

            // Calculate how much free space is left, the max of what we processed vs what we got back
            free_space -= processed.max(new_data.len());

            // Add the rle1 data to block.data
            block.data.extend(new_data.iter());
            timer.mark("rle1");

            // Update the block end to the actual block size
            block.end = block.data.len() as u32;

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
