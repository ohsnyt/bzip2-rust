use std::fs::{File, self};
use std::io::{self, Read, Write};

use super::bitwriter::BitWriter;
use super::compress_block::compress_block;
use super::crc::{do_crc, do_stream_crc};
use super::options::Status;
use super::options::Verbosity::Chatty;
use super::report::report;
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
    pub seq: u32,
    pub block_crc: u32,
    pub stream_crc: u32,
    pub bytes: u64,
    pub is_last: bool,
    pub block_size: u8,
}

/// These are the steps necessary to compress. Input file defined in options.
pub fn compress(opts: &mut BzOpts) -> io::Result<()> {
    // Create the struct to pass data to compress_block.rs
    // Initialize the size of the data vec to the block size to avoid resizing
    let mut bw = BitWriter::new();

    //let data = &vec![];
    let mut next_block = Block {
        //data,
        seq: 0,
        block_crc: 0,
        stream_crc: 0,
        bytes: 0,
        is_last: false,
        block_size: opts.block_size,
    };

    // Initialize stuff to read the file
    let _input = match data_in::init(opts) {
        Err(e) => {
            opts.status = Status::NoData;
            return Err(e);
        }
        Ok(input) => input,
    };

    // Prepare to read the data.
    let fname = opts.file.as_ref().unwrap().clone();
    let mut fin = File::open(&fname)?;
    let fin_metadata = fs::metadata(&fname)?;
    let mut fin_end = fin_metadata.len() as usize;

    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.file.as_ref().unwrap().clone();
    fname.push_str(".bz2");
    let mut f_out = File::create(&fname)?;

    //----- Loop through blocks of data and process it.
    loop {
        let mut buffer = vec![0_u8; (opts.block_size as usize) * 100000.min(fin_end as usize)];
        fin.read_exact(&mut buffer)?;
        fin_end -= buffer.len();
        if buffer.len() < (opts.block_size as usize) * 100000 {
            next_block.is_last = true
        }
        next_block.seq += 1;
        report(opts, Chatty, format!("Starting block {}", &next_block.seq));
        next_block.block_crc = do_crc(&buffer);
        next_block.stream_crc = do_stream_crc(next_block.stream_crc, next_block.block_crc);

        // Do the compression
        compress_block(opts, &buffer, &mut bw, &next_block);

        // Write out what we have so we don't have to hold it all.
        f_out.write_all(&bw.output)?;
        report(
            opts,
            Chatty,
            format!(
                "Wrote block. Length is {} bytes. CRC is {:08x}",
                &bw.output.len(),
                &next_block.block_crc
            ),
        );
        bw.output.clear();

        // Exit if we are all done.
        if next_block.is_last {
            break;
        }
        buffer.clear();
    }

    bw.flush();
    // Write the last of the data.
    f_out.write_all(&bw.output)?;
    report(opts, Chatty, "Finished writing the compressed file.");
    Ok(())
}
