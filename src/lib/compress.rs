use std::fs::{self, File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, Write};

use log::{info, trace, warn};

use crate::lib::crc::CRC;
use crate::lib::rle1::Encode;

use super::bitwriter::BitWriter;
use super::compress_block::compress_block;
use super::options::Status;
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
    trace!("Initializing BitWriter and block struct");
    // Create the struct to pass data to compress_block.rs
    // Initialize the size of the data vec to the block size to avoid resizing
    let mut bw = BitWriter::new(opts.block_size as usize * 100000);

    // NOTE: There is a LIKELY PROBLEM with the block size calculation.
    /* Julian took 19 off the block size. I'm taking 19 off for every 100k in.
    The problem can exist because if the first RLE effort expands the file, then the
    original bzip2 will not decompress it. It seems to need the data to be limited
    to less than 900k **at any time**.
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
    trace!("Getting command line parameters");
    let _input = match data_in::init(opts) {
        Err(e) => {
            opts.status = Status::NoData;
            return Err(e);
        }
        Ok(input) => input,
    };

    // Prepare to read the data.
    let fname = opts.file.as_ref().unwrap().clone();
    trace!("Preparing to get input file for reading ({})", fname);
    let fin = File::open(&fname)?;
    let _fin_metadata = fs::metadata(&fname)?;

    // Prepare to hold a block of data
    let mut block_data: Vec<u8> = Vec::with_capacity(opts.block_size as usize * 100000);
    // Prepare the CRC encoder
    let mut crc = CRC::new();
    // Prepare the RLE encoder
    let mut rle = Encode::new();

    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.file.as_ref().unwrap().clone();
    fname.push_str(".bz2");
    trace!("Opening output file for writing ({})", fname);
    let mut f_out = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&fname)?;

    //----- Loop through blocks of data and process it.
    for byte in fin.bytes() {
        let byte = byte.unwrap();
        // Update the block crc with this data
        crc.add_byte(byte);
        // Encode the data with the Run Length Encoder - which will return 0, 1, or 2 bytes
        if let (Some(byte), part_b) = rle.next(byte) {
            block_data.push(byte);
            if part_b.is_some() {
                block_data.push(part_b.unwrap());
            }
        }
        // Check if we have reached the maximum block size, but not in the middle of run
        if block_data.len() >= block.block_size && !rle.run {
            // Flush the rle encoder to make sure we get any dangling run count
            if let Some(byte) = rle.flush() {
                block_data.push(byte);
            }
            // We reached the max block size, so process this block
            // Update the block sequence counter and inform the user
            block.seq += 1;
            info!("Starting block {}", &block.seq);

            // Update and record the stream crc
            crc.update_stream_crc();
            block.stream_crc = crc.get_stream_crc();
            warn!(
                "Block crc is {}, stream crc is {}",
                crc.get_block_crc(),
                crc.get_stream_crc()
            );

            // Record the block crc
            block.block_crc = crc.get_block_crc();

            // Do the compression
            compress_block(&block_data, &mut bw, &block, opts.block_size);

            // Write out what we have so we don't have to hold it all.
            f_out.write_all(&bw.output)?;
            info!(
                "Wrote block. Bitstream length is {} bytes. CRC is {}.\n",
                &bw.output.len(),
                &block.block_crc
            );
            // clear the output buffer, block vec and block_crc for the next loop
            bw.output.clear();
            block_data.clear();
            crc.reset_block_crc();
        }
    }
    // We finished the input file before maximum size for a block, so compress this partial block
    // Flush the encoder at the end in case we are in the middle of a run.
    if let Some(byte) = rle.flush() {
        block_data.push(byte)
    }
    // Update the block sequence counter and inform the user
    block.seq += 1;
    info!("Starting block {}", &block.seq);

    // Update and record the stream crc
    crc.update_stream_crc();
    block.stream_crc = crc.get_stream_crc();
    warn!(
        "Block crc is {}, stream crc is {}",
        crc.get_block_crc(),
        crc.get_stream_crc()
    );

    // Record the block crc
    block.block_crc = crc.get_block_crc();

    // Inform block_compress that this is the last block
    block.is_last = true;
    // Then do the compression
    compress_block(&block_data, &mut bw, &block, opts.block_size);

    // Write out what we have so we don't have to hold it all.
    f_out.write_all(&bw.output)?;
    info!(
        "Wrote block. Bitstream length is {} bytes. Block CRC is {}.\n",
        &bw.output.len(),
        &block.block_crc
    );
    // All done with the input. Return Ok.
    Ok(())
}
