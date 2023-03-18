use super::compress_block::compress_block;
use crate::bitstream::bitwriter::BitWriter;
use crate::tools::{cli::BzOpts, rle1::RLE1Block};
use rayon::prelude::*;
use simplelog::info;
use std::fs::File;
use std::io;
use std::thread::sleep;

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
      Since this can be parallel, we pass a reference to the u8 data as well as a sequence number.
      We will receive back the compressed data and sequence number. We will then assemble the compressed
      data segments in the correct order and write them out to the output file.

      THE ROUTINES FOR FILE I/O ARE RUDEMENTARY, AND DO NOT PROPERLY RESOLVE ALL I/O ERRORS.
    */

    // Prepare to read the data.
    let fname = opts.files[0].clone();
    let source_file = File::open(&fname)?;

    // Initialize the RLE1 reader/iterator. This reads the input file and creates blocks of the
    // proper size to then be compressed.
    let block_size = (opts.block_size as usize * 100000) - 19;
    let rle1_blocks = RLE1Block::new(source_file, block_size);

    // Prepare to write the compressed data. 
    let mut fname = opts.files[0].clone();
    fname.push_str(".bz2");

    /*
    This works by compressing each block in parallel. Depending on the sequence of when those blocks finish,
    this will hold compressed blocks in memory until it is their turn to be written.
    */

    // Initialize thread channel communication. Sends block data, sequence number, and indicator whether
    //  this is the last block
    let (tx, rx) = std::sync::mpsc::channel();
    // Initialize a bitwriter.
    let mut bw = BitWriter::new(&fname, opts.block_size as u8);

    // Spawn the BitWriter thread and wait for blocks to write.
    let handle = std::thread::spawn(move || {
        // Set the current block (the block we are waiting to write) to 0.
        let mut current_block = 0;
        // Initialize a vec to hold out-of-sequence blocks we might receive
        let mut results = vec![];

        'outer: loop {
            info!(
                "RX: Waiting for block {}. The queue contains {} blocks.",
                current_block,
                results.len(),
            );

            // Wait for a block to be sent to this thread.
            let result: ((Vec<u8>, u8), usize, bool) = rx.recv().unwrap();
            // If the block is the one we are waiting for, process it.
            if result.1 == current_block {
                info!("RX: Found block {}. Writing it...", current_block,);
                let data = &result.0 .0;
                let padding = result.0 .1;
                let last = result.2;
                bw.add_block(last, data, padding).unwrap();
                current_block += 1;
                if last {
                    break;
                }
            } else {
                info!(
                    "RX: Adding block {} to the queue. The queue will now contain {} blocks.",
                    result.1,
                    results.len() + 1,
                );
                // Otherwise, save it until we get the one we want.
                results.push(result);
            }
            while let Some(idx) = results.iter().position(|x| x.1 == current_block) {
                info!("RX: Found block {}. Writing it...", current_block,);
                let data = &results[idx].0 .0;
                let last_bits = results[idx].0 .1;
                let last = results[idx].2;
                bw.add_block(last, data, last_bits).unwrap();
                results.swap_remove(idx);
                current_block += 1;
                if last {
                    break 'outer;
                }
            }
        }
    });

    // Build the RLE1 blocks and compress them
    rle1_blocks
        .into_iter()
        .enumerate()
        .par_bridge()
        .for_each_with(tx, |tx, (i, (crc, block, last_block))| {
            let result = compress_block(&block, crc);
            tx.send((result, i, last_block)).unwrap();
        });
    let joined =  handle.join();
    info!("RX: Thread returned {:?}", joined);
    Ok(())
}
