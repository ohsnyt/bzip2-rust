use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};

use log::{info, trace};

use super::bitwriter::BitWriter;
use super::compress_block::compress_block;
use super::crc::{do_crc, do_stream_crc};
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
    pub bytes_to_go: usize,
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

    //let data = &vec![];
    let mut block = Block {
        bytes_to_go: 0,
        block_size: opts.block_size as usize * 100000 - 19, // ds, Julian took off 19 for nblockMAX (60 in real life exampleopts.block_size)
        seq: 0,
        block_crc: 0,
        stream_crc: 0,
        is_last: false,
    };

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
    let mut fin = File::open(&fname)?;
    let fin_metadata = fs::metadata(&fname)?;
    block.bytes_to_go = fin_metadata.len() as usize;

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
    loop {
        // set an appropriate sized buffer for the block size
        let mut buffer = vec![0_u8; block.block_size.min(block.bytes_to_go)];
        // read data, which may read much less than the buffer length
        let bytes_read = fin.read(&mut buffer)?;
        // adjust the buffer length down to what we read.
        buffer.truncate(bytes_read);
        // check if we are at the end of the input file (fin). If so, is_last = true.
        block.bytes_to_go -= bytes_read;
        if block.bytes_to_go == 0 {
            block.is_last = true
        }
        trace!(
            "Block {} holds {} bytes{}.",
            block.seq,
            bytes_read,
            if block.is_last {
                " and is the last block"
            } else {
                ""
            }
        );

        // update the block sequence counter
        block.seq += 1;
        info!("Starting block {}", &block.seq);
        block.block_crc = do_crc(&buffer);
        block.stream_crc = do_stream_crc(block.stream_crc, block.block_crc);

        // Do the compression
        compress_block(&buffer, &mut bw, &block, opts.block_size);

        // Write out what we have so we don't have to hold it all.
        f_out.write_all(&bw.output)?;
        info!(
            "Wrote block. Bitstream length is {} bytes. CRC is {}.\n",
            &bw.output.len(),
            &block.block_crc
        );
        // clear the output buffer
        bw.output.clear();

        // Exit if we are at the end of the file, else loop again.
        if block.is_last {
            info!("Finished writing the compressed file.");
            break;
        }    
    }

    Ok(())
}
