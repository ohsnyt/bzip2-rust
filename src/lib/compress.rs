use std::fs::File;
use std::io::Write;

//use super::bitreader::BitReader;
use super::bitwriter::BitWriter;
use super::compress_block::compress_block;
use super::crc::do_crc;
use super::options::Status;
use super::options::Verbosity::Chatty;
//use super::options::Verbosity::Errors;
//use super::options::Verbosity::Normal;
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
    pub bw: BitWriter,
    pub data: Vec<u8>,
    pub seq: u32,
    pub block_crc: u32,
    pub stream_crc: u32,
    pub bytes: u64,
    pub is_last: bool,
    pub block_size: u8,
}
impl Block {
    fn new(block_size: u8) -> Self {
        Self {
            bw: BitWriter::new(),
            data: Vec::with_capacity((block_size * 100000) as usize),
            seq: 0,
            block_crc: 0,
            stream_crc: 0,
            bytes: 0,
            is_last: false,
            block_size,
        }
    }
}
/// These are the steps necessary to compress. Input file defined in options.
pub fn compress(opts: &BzOpts) {
    // Create the struct to pass data to compress_block.rs
    // Initialize the size of the data vec to the block size to avoid resizing
    let mut next_block = Block::new(opts.block_size);

    // Initialize stuff to read the file
    let mut reader = match data_in::init(&opts) {
        Err(_) => {
            opts.status = Status::NoData;
            return;
        }
        Ok(reader) => reader,
    };
    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.file.as_ref().unwrap().clone();
    fname.push_str(".bz2");
    let mut f_out =
        File::create(&fname).expect(&format!("Unable to create compressed file {}", &fname));

    //----- Loop through blocks of data and process it.
    // Try to get a block of data
    loop {
        let (data, last) = reader.read();
        match data {
            // Got some. Set up next block, do CRC, etc.
            Some(d) => {
                next_block.data = *d;
                report(&opts, Chatty, format!("Starting block {}", next_block.seq));
                next_block.block_crc = do_crc(d);
                report(
                    &opts,
                    Chatty,
                    format!(
                        "Block length is {} bytes. CRC is {:08x}",
                        d.len(),
                        next_block.block_crc
                    ),
                );
                next_block.stream_crc =
                    (next_block.stream_crc << 1) | (next_block.stream_crc >> 31);
                next_block.stream_crc ^= next_block.block_crc;
                next_block.seq += 1;
            }
            // Oops, no data.
            None => next_block.data = vec![],
        };
        // Let the compress_block know if this is the last block
        next_block.is_last = last;
        // Do the compression
        compress_block(&next_block);
        // Exit if we are all done.
        if next_block.is_last {
            break;
        }
    }
    // Actually write out the data. This perhaps should be
    //  part of the loop so we don't have to hold it all.
    f_out
        .write_all(&next_block.bw.output)
        .expect(&format!("Unable to write compressed file {}", &fname));
    report(&opts, Chatty, "Finished writing the compressed file.");
}
