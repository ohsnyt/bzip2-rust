use log::{error, info, trace, warn};

use crate::{
    snyder::bwt_ds::bwt_decode_test,
    tools::{
        crc::{do_crc, do_stream_crc},
        rle2_mtf_decode::rle2_mtf_decode_fast,
    },
    Timer,
};

use crate::bitstream::bitreader::BitReader;

use crate::tools::{cli::BzOpts, rle1::rle1_decode, symbol_map::decode_sym_map};

use std::{
    fs::{File, OpenOptions},
    io::{self, Error, Write},
};

//const BUFFER_SIZE: usize = 100000;
const EOF_MESSAGE: &str = "Unexpected End Of File";
const CHUNK_SIZE: usize = 50; // Bzip2 chunk size
const FOOTER: [u8; 6] = [0x17, 0x72, 0x45, 0x38, 0x50, 0x90];
const HEADER: [u8; 6] = [0x31_u8, 0x41, 0x59, 0x26, 0x53, 0x59];

/// Decompress the file specified in opts (BzOpts). Current version also requires a Timer.
pub(crate) fn decompress(opts: &BzOpts, timer: &mut Timer) -> io::Result<()> {
    // Start bitreader from input file in the command line
    let mut br = BitReader::new(File::open(opts.file.as_ref().unwrap())?);

    // We will eventually need to mark the output file with the timestamp of the compresssed file.
    //let metadata = std::fs::metadata(opts.file.as_ref().unwrap().to_string())?;

    // Look for a valid signature.
    if "BZh".as_bytes() == br.bytes(3).expect(EOF_MESSAGE) {
        info!("Found a valid bzip2 signature.");
    } else {
        error!(
            "Fatal error: {} is not a valid bzip2 compressed file.",
            opts.file.as_ref().unwrap()
        );
        return Err(Error::new(io::ErrorKind::Other, "Invalid compressed file."));
    }

    // Use the block size to validate the max number of selectors.
    // (Use saturating_sub in case there is a data error - to prevent underflow)
    let block_size = br.byte().expect(EOF_MESSAGE).saturating_sub(0x30);
    if !(1..=9).contains(&block_size) {
        error!("Fatal error: Found invalid block size.");
        return Err(Error::new(io::ErrorKind::Other, "Invalid block size"));
    }

    // Good so far. Prepare to write the data. (Drop temp variables after this block)
    let mut f_out: File;
    {
        let mut fname = opts.file.as_ref().unwrap().clone();
        fname = fname.split(".bz2").map(|s| s.to_string()).collect(); // strip off the .bz2
        fname.push_str(".txt"); // for my testing purposes.
        f_out = OpenOptions::new()
            .write(true)
            .create(true)
            //.append(true)
            .open(&fname)
            .expect("Can't open file for writing.");
    }
    timer.mark("setup");

    // Initialize steam CRC value
    let mut stream_crc = 0;
    // Initialize block_counter for reporting purposes
    let mut block_counter = 0;
    // Save space for the symbol set
    let mut symbol_set: Vec<u8>;
    let mut symbols: usize;

    'block: loop {
        block_counter += 1;

        // Block header (or footer) should come next.
        if let Some(header_footer) = br.bytes(6) {
            // Check for footer first. Exit the loop block when we find it.
            if header_footer == FOOTER {
                break 'block;
            }
            // We must now have a block header. Create an error if not.
            if header_footer != HEADER {
                return Err(Error::new(io::ErrorKind::Other, "Invalid block footer"));
            }
            info!("Found a valid header for block {}.", block_counter);
        };

        // Get crc
        let block_crc = br.bint(32).expect(EOF_MESSAGE);
        info!("CRC is {}.", block_crc);

        // Get randomize flag - should almost always be zero
        let rand = br.bool_bit().expect(EOF_MESSAGE);
        trace!("\nRandomized is {:?}.", rand);

        // Get key (origin pointer)
        let key = br.bint(24).expect(EOF_MESSAGE);
        if key > block_size as usize * 100000 + 10 {
            error!("Invalid key pointer");
            return Err(Error::new(io::ErrorKind::Other, "Invalid key pointer"));
        }
        info!("Key is {}.", key);

        // Get the symbol info. (Use block to drop the temporary vec used to grab the data)
        {
            // First set up a temporary map vec starting with the map "index".
            let mut sym_map: Vec<u16> = vec![br.bint(16).expect(EOF_MESSAGE) as u16];

            // Now get as many 16-symbol maps as indicated by the set bits in the "index"
            let symbol_loc = br.loc();
            for _i in 0..sym_map[0].count_ones() as usize {
                sym_map.push(br.bint(16).expect(EOF_MESSAGE) as u16);
            }

            // Decode the symbol map and save it
            symbol_set = decode_sym_map(&sym_map);
            //symbol_set = symbol_set[1..symbol_set.len()].to_vec();

            // Count how many symbols are in the symbol map. The +2 adds in RUNA / RUNB plus EOB.
            symbols = symbol_set.len() + 1;
            info!("Found {} symbols for block {}.", symbols, block_counter);
            trace!(
                "\nFound {} symbols for block {} at {}.",
                symbol_set.len(),
                block_counter,
                symbol_loc
            );
        }

        // Read NumTrees
        let table_count = br.bint(3).expect(EOF_MESSAGE);
        if !(2..=6).contains(&table_count) {
            error!("Invalid table count");
            return Err(Error::new(io::ErrorKind::Other, "Invalid table count"));
        }

        // Read Selector_count (NumSels in Julian speak) (mutable, because we may need to adjust it)
        let mut selector_count = br.bint(15).expect(EOF_MESSAGE);

        // Read Selectors based on the actual number of selectors reported
        // (But only save the ones we can use! Hence max_selectors.)
        let mut selector_map = vec![0_usize; selector_count];
        // Use block to drop temporary variables
        {
            // First read the "raw" selector map
            let mut raw_selector_map = Vec::with_capacity(selector_count as usize);
            // Set selector maximum
            let max_selectors = block_size as usize * 100000 / 50;
            let mut group: u8 = 0;
            for _ in 0..selector_count {
                while br.bool_bit().expect(EOF_MESSAGE) {
                    group += 1;
                }
                // Like Julian, ignore  excessive selectors, only push maps that can be used.
                if selector_count <= max_selectors {
                    raw_selector_map.push(group);
                }
                group = 0;
            }
            // Adjust the selector_count if needed. This should never happen.
            if selector_count > max_selectors {
                warn!("Found {} selectors were reported, but the maximum is {}. Adjust the selector count down.", selector_count, max_selectors);
                selector_count = max_selectors;
            }

            // Time to reverse the MTF on the selectors that we received
            // Create an index vec for the number of tables we need
            let mut table_idx: Vec<usize> = (0..table_count as usize).collect();
            // Undo the move to the front.
            //----------------------------------------------------------------
            // iterate through the input
            for (i, &selector) in raw_selector_map.iter().enumerate() {
                // Create index from the selector
                let mut idx = selector as usize;

                // Save the selector from the MTF index
                selector_map[i] = table_idx[idx];

                // Check if the data is correct
                // if check(i) != table_idx[idx] {
                //     println!("Element {}: {} != {}.  Pause here...", i, idx, check(idx))
                // };

                // Shift each index at the front of mtfa "forward" one. Do this first in blocks for speed.
                let temp_sym = table_idx[idx];

                while idx > 2 {
                    table_idx[idx] = table_idx[idx - 1];
                    table_idx[idx - 1] = table_idx[idx - 2];
                    table_idx[idx - 2] = table_idx[idx - 3];
                    idx -= 3;
                }
                // ...then clean up any odd ones
                while idx > 0 {
                    table_idx[idx] = table_idx[idx - 1];
                    idx -= 1;
                }
                // ...and finally move this index to the front.
                table_idx[0] = temp_sym;
            }

            info!(
                "Decoded {} selectors for the {} tables in block {}.",
                selector_count, table_count, block_counter
            );
        }

        timer.mark("setup");

        // Read the Huffman symbol lengths and create decode maps which have decoding info
        //  and a level-specific vec of the symbols.

        let mut huf_decode_maps: Vec<(Vec<Level>, Vec<u16>)> =
            vec![(Vec::new(), Vec::with_capacity(symbols)); table_count];

        for huffman_decode_map in huf_decode_maps.iter_mut().take(table_count) {
            // Tracing info
            let mark_loc = br.loc();

            // Create a temporary vec for the next map
            let mut map: Vec<(u16, u32)> = vec![(0_u16, 0_u32); symbols + 1];
            // Read the origin length - five bits long
            let mut l: i32 = br.bint(5).expect(EOF_MESSAGE) as i32;
            // For each known symbol at this level (including a repeat of the origin we just read)
            // calculate the symbol length based on the relative bit length from the base symbol we just read.
            for symbol in 0..symbols as u16 + 1 {
                let mut diff: i32 = 0;
                //loop {
                // Look for offset pairs
                while br.bool_bit().expect(EOF_MESSAGE) {
                    // Get the second bit. If it is a 1, subract 1 from diff. Otherwise add one to diff.
                    if br.bool_bit().expect(EOF_MESSAGE) {
                        diff -= 1 // Found "11" - subtract 1
                    } else {
                        diff += 1 // Found "10" - add 1
                    }
                }
                // No more offsets. Calculate the total offset and map the symbol.
                map[symbol as usize] = (symbol, (l + diff) as u32);
                if l + diff > 17 {
                    error!(
                        "Symbol length of {} exceeds max for sym {} in table {}",
                        l + diff,
                        symbol,
                        table_count
                    );
                    //return Err(Error::new(io::ErrorKind::Other, "Invalid symbol length"));
                }
                // The next code is calculated offset from the length of the symbol we just decoded.
                l += diff;
                //break;
            }
            //}
            //}
            // Maps must be sorted by length for the next step.
            map.sort_by(|a, b| a.1.cmp(&b.1));

            // Build the decode map and store it along with the symbol list for decoding this map.
            *huffman_decode_map = (
                huf_decode_map(&map),
                map.iter().map(|(s, _)| *s).collect::<Vec<u16>>(),
            );
            trace!("\rFound huffman maps at {}.  ", mark_loc);
        }

        // We are now ready to read the data and decode it.
        // Set aside a output vec to store the data we decode (size based on the table count)
        let mut out = vec![
            0_u16;
            match table_count {
                2 => 200,
                3 => 600,
                4 => 1200,
                5 => 2400,
                _ => (block_size as usize * 100000) + 19,
            }
        ];

        // Now read the input block in chunks of 50 symbols using the huffman map for that chunk indicated by the selector map
        {
            // Isolate temporary variable in this block.
            // Initialize key variables
            let mut block_index = 0;
            //let mut bit_count: u32 = 0;
            let mut code = 0_u32;
            let mut depth = 0;
            // Set the eob symbol.
            let eob = symbols as u16;

            // Get references to the current level variables and symbol set.
            //   Too bad we have to do a "double" assignment here and about line 375.
            let (l, s) = &huf_decode_maps[selector_map[block_index]];
            let mut level = l;
            let mut symbol_index = s;

            // Loop through the data in chunks trying to find valid symbols in the bit stream
            loop {
                // Left shift any code bits we are currently holding so we can add in the next level of bits
                code <<= level[depth].bits;

                // Get the required bits at this level depth and add them to our code
                //time.mark("h_bitread");
                code |= br.bint(level[depth].bits as usize).expect(EOF_MESSAGE) as u32;
                //time.mark("huffman");

                // If the code is bigger than the end code at this level, try the next level
                if code >= level[depth].end_code {
                    depth += 1;
                    continue;
                } else {
                    // We found a code in this level. Calculate the offset and grab the symbol
                    let sym = symbol_index
                        [(level[depth].offset + code - level[depth].start_code) as usize];

                    // Put it into the output vec.
                    out[block_index] = sym;
                    trace!("\r\x1b[43m{:>6}: {:>3}     \x1b[0m", block_index, sym);

                    // Check if we have reached the end of block
                    if sym == eob {
                        // If we are, check if we are at the end of the block too early
                        if block_index / CHUNK_SIZE < selector_count as usize - 1 {
                            error!("Found EOB before working through all selectors. (Chunk {} instead of {}.)", block_index/50, selector_count);
                            return Err(Error::new(
                                io::ErrorKind::Other,
                                "Found end of block too early",
                            ));
                        }
                        // Adjust the vec length to the block_index plus 1
                        out.truncate(block_index + 1);
                        // All done.
                        break;
                    }

                    // Update the block index
                    block_index += 1;

                    // Update the level variables if we are starting a new chunk.
                    if block_index % CHUNK_SIZE == 0 {
                        let (l, s) = &huf_decode_maps[selector_map[block_index / 50]];
                        level = l;
                        symbol_index = s;
                    }

                    // Reset the depth index and code before looking for the next symbol.
                    depth = 0;
                    code = 0;
                }
            }
        }
        timer.mark("huffman");

        // Undo the RLE2 and MTF, converting to u8 in the process
        // Set aside a vec to store the data we decode (size based on the table count)
        let size = 900019_usize;

        let (mtf_out, freq) = rle2_mtf_decode_fast(&out, &mut symbol_set, size);

        timer.mark("rle_mtf");

        // Undo the BWTransform
        let bwt_v = bwt_decode_test(key as u32, &mtf_out, &freq);
        trace!("{:?}", String::from_utf8(bwt_v.clone()));
        //let mut bwt_v = crate::lib::bwt_ds::bwt_decode_fastest(key as u32, &mtf_8); //, &symbol_set);

        timer.mark("bwt");

        // Undo the initial RLE1
        let rle1_v = rle1_decode(&bwt_v);
        trace!("{:?}", String::from_utf8(rle1_v.clone()));

        timer.mark("rle1");

        // Compute and check the CRCs
        let this_block_crc = do_crc(0, &rle1_v);
        //let this_block_crc = CRC32.checksum(&rle1_v);
        stream_crc = do_stream_crc(stream_crc, this_block_crc);

        if block_crc == this_block_crc as usize {
            info!("Block {} CRCs matched.", block_counter);
        } else {
            error!(
                "Block {} CRC failed!!! Found {} looking for {}. (Continuing...)",
                block_counter,
                this_block_crc,
                block_crc // Perhaps this should be a fatal error as the data is corrupt.
            );
        }

        timer.mark("crcs");

        // Done!! Write the data.
        let result = f_out.write(&rle1_v);
        info!("Wrote a block of data with {} bytes.", result.unwrap());

        timer.mark("cleanup");
    }

    let final_crc = br.bint(32).expect(EOF_MESSAGE);
    if final_crc == stream_crc as usize {
        info!("Stream CRCs matched: {}.", final_crc);
    } else {
        error!(
            "Stream CRC failed!!! Found {} looking for {}. (Data may be corrupt.)",
            stream_crc, final_crc
        );
        // Perhaps this should be a fatal error as the data is corrupt.
        // This should never happen unless a block CRC also failed - or unless there is a missing block.
    }
    timer.mark("cleanup");

    Result::Ok(())
}

#[derive(Debug, Clone)]
struct Level {
    bits: u32,
    offset: u32,
    start_code: u32,
    end_code: u32,
}
impl Level {
    fn new() -> Self {
        Self {
            bits: 0,
            offset: 0,
            start_code: 0,
            end_code: 0,
        }
    }
}

/// Decode a vec of symbols and lengths into the level structure needed to efficiently
/// decode the bit stream.
fn huf_decode_map(map: &[(u16, u32)]) -> Vec<Level> {
    // Initialize result vector
    let mut result = Vec::new();

    // Current_length is the number of bits sured for the code length at this level
    let mut current_bit_length = map[0].1;

    // Bits_to_add is the number of bits we need to check codes at this level. (First time
    // it is also the bit length of the code)
    let mut bits_to_add = current_bit_length;

    // Current_code is the starting code at this level
    let mut current_code = 0;

    // Set symbol count variables
    let mut count = 0_u32;
    let mut last_count = count;

    // For each bit level (number of bits in the code), get the symbol list
    for (_symbol, bit_length) in map.iter() {
        count += 1;
        if *bit_length == current_bit_length {
            continue;
        } else {
            // Done at this level. Record the level.
            let mut level = Level::new();

            level.bits = bits_to_add;
            level.offset = last_count;
            level.start_code = current_code;
            level.end_code = (current_code + count - 1) as u32;
            result.push(level);

            // Calculate the number of bits needed to get to the next level
            bits_to_add = bit_length - current_bit_length;

            // Update current_code for the next iteration before we change count
            current_code = (current_code + count - 1) << bits_to_add;

            // Update last_count for the next iteration
            last_count += count - 1;

            // Reset count to 1 (because we counted one already)
            count = 1;

            // Set current_length for the next level
            current_bit_length = *bit_length;
        }
    }
    // Done at the last level. Record the level information.
    let mut level = Level::new();
    //let symbol = map[map.len() - 1].0;

    level.bits = bits_to_add;
    level.offset = last_count;
    level.start_code = current_code;
    level.end_code = current_code + count as u32;
    result.push(level);

    result
}
