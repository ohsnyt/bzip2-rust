use log::{debug, error, info, warn};

use crate::lib::crc::{do_crc, do_stream_crc};
use rustc_hash::FxHashMap;

use super::{
    bitreader::BitReader,
    //bwt_ds::bwt_decode,
    //bwt_inverse::inverse_bwt,
    mtf::mtf_decode,
    options::BzOpts,
    rle1::rle1_decode,
    rle2::rle2_decode,
    symbol_map::{self, decode_sym_map},
};

use std::{
    //collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, Error, Write},
    time::{Duration, Instant},
};

struct Timer {
    setup: Duration,
    huffman: Duration,
    rle_mtf: Duration,
    bwt: Duration,
    rle1: Duration,
    cleanup: Duration,
    total: Duration,
    time: Instant,
}
impl Timer {
    fn new() -> Self {
        Self {
            setup: Duration::new(0, 0),
            huffman: Duration::new(0, 0),
            rle_mtf: Duration::new(0, 0),
            bwt: Duration::new(0, 0),
            rle1: Duration::new(0, 0),
            cleanup: Duration::new(0, 0),
            total: Duration::new(0, 0),
            time: Instant::now(),
        }
    }
    fn mark(&mut self, area: &str) {
        match area {
            "setup" => {
                self.setup += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
            "huffman" => {
                self.huffman += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
            "rle_mtf" => {
                self.rle_mtf += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
            "bwt" => {
                self.bwt += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
            "rle1" => {
                self.rle1 += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
            _ => {
                self.cleanup += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
        }
    }
}

const BUFFER_SIZE: usize = 100000;
const EOF_MESSAGE: &str = "Unexpected End Of File";
const CHUNK_SIZE: usize = 50; // Bzip2 chunk size
const FOOTER: [u8; 6] = [0x17, 0x72, 0x45, 0x38, 0x50, 0x90];
const HEADER: [u8; 6] = [0x31_u8, 0x41, 0x59, 0x26, 0x53, 0x59];

/// Decompress the file given in the command line
pub(crate) fn decompress(opts: &BzOpts) -> io::Result<()> {
    // DEBUG timer
    let mut time = Timer::new();

    // Initialize steam CRC value
    let mut stream_crc = 0;

    // Initialize stuff to read the file
    let mut f = "test.txt.bz2".to_string();
    if opts.file.is_none() {
        warn!("Using >test.txt.bz2< as the input file.");
    } else {
        f = opts.file.as_ref().unwrap().to_string()
    }

    //let file = File::open(&f)?;
    //let metadata = std::fs::metadata(&f)?;

    let mut br = BitReader::new(File::open(&f)?);
    //debug!("Starting decompression at {}", br.loc());

    // Look for a valid signature.
    if "BZh".as_bytes() == br.bytes(3).expect(EOF_MESSAGE) {
        info!("Found a valid bzip2 signature.");
    }

    // Use the block size to validate the max number of selectors.
    let mut block_size = br.byte().expect(EOF_MESSAGE);
    if !(0x30..=0x39).contains(&block_size) {
        return Err(Error::new(io::ErrorKind::Other, "Invalid block size"));
    }
    // Convert block_size to an integer for later use
    block_size -= 0x30;
    // Save space for the symbol set for later use
    let mut symbol_set: Vec<u8>;

    // Good so far. Prepare to write the data.
    let mut fname = opts.file.as_ref().unwrap().clone();
    fname = fname.split(".bz2").map(|s| s.to_string()).collect(); // strip off the .bz2
    fname.push_str(".txt"); // for my testing purposes.
    let mut f_out = OpenOptions::new()
        .write(true)
        .create(true)
        //.append(true)
        .open(&fname)
        .expect("Can't open file for writing.");

    // Block_counter is for reporting purposes
    let mut block_counter = 0;
    'block: loop {
        block_counter += 1;

        // Block header (or footer) should come next.
        if let Some(header_footer) = br.bytes(6) {
            //check for footer first
            if header_footer == FOOTER {
                break 'block;
            }
            // Then create an error if this is not a block header
            if header_footer != HEADER {
                return Err(Error::new(io::ErrorKind::Other, "Invalid block footer"));
            }
        };
        info!("Found a valid header for block {}.", block_counter);

        // Get crc
        let block_crc = br.bint(32).expect(EOF_MESSAGE);
        info!("CRC is {}.", block_crc);

        // Get randomize flag - should almost always be zero
        let rand = br.bool_bit().expect(EOF_MESSAGE);
        debug!("Randomized is {:?}.", rand);

        // Get key (origin pointer)
        let key = br.bint(24).expect(EOF_MESSAGE);
        if key > block_size as usize * 100000 + 10 {
            return Err(Error::new(io::ErrorKind::Other, "Invalid key pointer"));
        }
        info!("Key is {}.", key);

        // Get the symbol set (dropping the temporary vec used to grab the data)
        {
            // First get the map "index" and save it as the first entry in the map
            let mut sym_map: Vec<u16> = vec![0_u16; 17];
            sym_map[0] = br.bint(16).expect(EOF_MESSAGE) as u16;
            // Then get as many 16-symbol maps as indicated by the set bits in the "index"
            for i in 1..sym_map[0].count_ones() as usize {
                sym_map[i] = br.bint(16).expect(EOF_MESSAGE) as u16;
            }
            // Reduce sym_map down to the valid entries
            sym_map.truncate((sym_map[0].count_ones() as usize + 1));

            // Then decode the symbol map and save it
            symbol_set = decode_sym_map(&sym_map);
        }
        //  Count how many symbols are in the symbol map. The +2 adds in RUNA and RUNB.
        let symbols = symbol_set.len() + 2;
        info!("Found {} symbols for block {}.", symbols, block_counter);

        // Read NumTrees
        let table_count = br.bint(3).expect(EOF_MESSAGE);
        if !(2..=6).contains(&table_count) {
            return Err(Error::new(io::ErrorKind::Other, "Invalid table count"));
        }

        // Read Selector_count (NumSels in Julian speak) (mutable, because we may need to adjust it)
        let mut selector_count = br.bint(15).expect(EOF_MESSAGE);

        // Read Selectors based on the actual number of selectors reported
        let mut decode_map_selectors = Vec::with_capacity(selector_count as usize);
        let mut group: u8 = 0;
        for _ in 0..selector_count {
            while br.bool_bit().expect(EOF_MESSAGE) {
                group += 1;
            }
            // Since Julian ignores the error of excessive selector_count, only push maps that can be used
            if selector_count <= block_size as usize * 100000 / 50 {
                decode_map_selectors.push(group);
            }
            group = 0;
        }
        // Adjust the selector_count if needed
        if selector_count > block_size as usize * 100000 / 50 {
            warn!("Found {} selector were reported, but the maximum is {}. Adjust the selector count down.", selector_count, block_size as u32 * 100000 / 50);
            selector_count = block_size as usize * 100000 / 50;
        }

        // Decode selectors from MTF values for the selectors
        // Create an index vec for the number of tables we need
        let mut table_idx: Vec<usize> = (0..table_count as usize).collect();

        // Now undo the move to front for the selectors
        let decode_map_selectors = decode_map_selectors
            .iter()
            .fold((Vec::new(), table_idx), |(mut o, mut s), x| {
                o.push(s[*x as usize]);
                let c = s.remove(*x as usize);
                s.insert(0, c);
                (o, s)
            })
            .0;
        info!(
            "Decoded the {} selectors for the {} tables in block {}.",
            selector_count, table_count, block_counter
        );

        time.mark("setup");

        // Read the Huffman symbol length maps and create decode maps which have level info and a vec of the symbols
        let mut huf_decode_maps: Vec<(Vec<Level>, Vec<u16>)> =
            vec![(Vec::new(), Vec::with_capacity(symbols)); table_count];

        for idx in 0..table_count {
            let mut map: Vec<(u16, u32)> = vec![(0_u16, 0_u32); symbols];
            let mut l: i32 = br.bint(5).expect(EOF_MESSAGE) as i32;

            for symbol in 0..symbols as u16 {
                let mut diff: i32 = 0;
                loop {
                    if br.bool_bit().expect(EOF_MESSAGE) {
                        if br.bool_bit().expect(EOF_MESSAGE) {
                            diff -= 1
                        } else {
                            diff += 1
                        }
                    } else {
                        map[symbol as usize] = (symbol, (l + diff) as u32);
                        if l + diff > 17 {
                            warn!(
                                "Symbol length of {} exceeds max for sym {} in table {}",
                                l + diff,
                                symbol,
                                table_count
                            )
                        }
                        l += diff;
                        break;
                    }
                }
            }
            // Maps must be sorted by length for the next step.
            map.sort_by(|a, b| a.1.cmp(&b.1));
            // Create a symbol list for decoding
            let symbol_index = map.iter().map(|(s, _)| *s).collect::<Vec<u16>>();
            // Build the decode map and store it.
            huf_decode_maps[idx] = (decode_map(&map), symbol_index);
        }

        // We are now ready to read the data and decode it.
        // Set aside a vec to store the data we decode (size based on the table count)
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

        // Set the eob to one more than the symbols in symbol_set.
        let eob = symbol_set.len() as u16 + 1;

        // Now read the blocks in chunks of 50 symbols with the huffman map selected by the selector vec
        // Initialize key variables
        let mut block_index = 0;
        let mut bit_count: u32 = 0;
        let mut code = 0_u32;
        let mut depth = 0;

        // Get reference to the current level variables
        //let mut level: &Vec<Level>;
        //let mut symbol_index: &Vec<u16>;

        let (l, s) = &huf_decode_maps[decode_map_selectors[block_index]];
        let mut level = l;
        let mut symbol_index = s;
        // Loop through the data in chunks trying to find valid symbols in the bit stream
        loop {
            // Left shift any bits so we can add in one more bit
            code <<= level[depth].bits;

            // Get the required bits at this level depth and add them to our bits
            if let Some(bits) = br.bint(level[depth].bits as usize) {
                code |= bits as u32
            }

            // If the code is bigger than the end code at this level, try the next level
            if code >= level[depth].end_code {
                depth += 1;
                continue;
            } else {
                // We found a code in this level. Calculate the offset and grab the symbol
                let sym =
                    symbol_index[(level[depth].offset + code - level[depth].start_code) as usize];

                // Put it into the output vec.
                out[block_index] = sym;

                // Update the block index
                block_index += 1;

                // Update the level variables if we are in a new chunk.
                if block_index % CHUNK_SIZE == 0 {
                    let (l, s) = &huf_decode_maps[decode_map_selectors[block_index / 50]];
                    level = l;
                    symbol_index = s;
                }

                // Reset the depth index and code.
                depth = 0;
                code = 0;

                // Check if we have reached the end of block
                if sym == eob {
                    // Check if we are at the end of the block too early
                    if block_index / CHUNK_SIZE < selector_count as usize - 1 {
                        error!("Found EOB before working through all selectors. (Chunk {} instead of {}.)", block_index/50, selector_count)
                    }
                    // Adjust the vec length to the block_index
                    out.truncate(block_index);
                    // Break/Return with the block data
                    break;
                }
            }
        }

        time.mark("huffman");

        // Undo the RLE2 and MTF, converting to u8 in the process
        // Set aside a vec to store the data we decode (size based on the table count)
        let mut mtf_out: Vec<u8> = vec![
            0_u8;
            match table_count {
                2 => 200,
                3 => 600,
                4 => 1200,
                5 => 2400,
                _ => (block_size as usize * 100000) + 19,
            }
        ];
        rle2_mtf_decode(&out, &mut mtf_out, &mut symbol_set);

        time.mark("rle_mtf");

        // Undo the BWTransform
        let mut bwt_v = crate::lib::bwt_ds::bwt_decode_small(key as u32, &mtf_out); //, &symbol_set);
        let first_byte = bwt_v[0];
        //bwt_v.remove((0));
        //bwt_v.push(first_byte);

        time.mark("bwt");

        // Undo the initial RLE1
        let rle1_v = rle1_decode(&bwt_v);

        time.mark("rle1");

        // Compute the CRC
        let this_block_crc = do_crc(0, &rle1_v);
        stream_crc = do_stream_crc(stream_crc, this_block_crc);

        if block_crc == this_block_crc as usize {
            info!("Block {} CRCs matched.", block_counter);
        } else {
            error!(
                "Block {} CRC failed!!! Found {} looking for {}. (Continuing...)",
                block_counter, this_block_crc, block_crc
            );
        }

        // Done!! Write the data
        let result = f_out.write(&rle1_v);
        info!("Wrote a block of data with {} bytes.", result.unwrap());

        time.mark("cleanup");
    }

    let final_crc = br.bint(32).expect(EOF_MESSAGE);
    if final_crc == stream_crc as usize {
        info!("Stream CRCs matched: {}.", final_crc);
    } else {
        error!(
            "Stream CRC failed!!! Found {} looking for {}. (Data may be corrupt.)",
            stream_crc, final_crc
        );
    }

    info!("Wrote the decompressed file.\n");

    time.mark("rle_cleanup");
    println!("Setup:\t\t{:?}", time.setup);
    println!("Huffman:\t{:?}", time.huffman);
    println!("RLE/MTF:\t{:?}", time.rle_mtf);
    println!("BWT\t\t{:?}", time.bwt);
    println!("RLE1:\t\t{:?}", time.rle1);
    println!("Cleanup:\t{:?}", time.cleanup);
    println!("Total:\t\t{:?}", time.total);

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

fn decode_map(map: &Vec<(u16, u32)>) -> Vec<Level> {
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
    for (symbol, bit_length) in map.iter() {
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
    // Done at the last level. Record the level.
    let mut level = Level::new();
    let symbol = map[map.len() - 1].0;

    level.bits = bits_to_add;
    level.offset = last_count;
    level.start_code = current_code;
    level.end_code = current_code + count as u32;
    result.push(level);

    result
}

const RUNA: u16 = 0;
const RUNB: u16 = 1;

/// Does run-length-decoding from rle2_encode.
pub fn rle2_mtf_decode(data_in: &[u16], out: &mut Vec<u8>, mut mtf_index: &mut Vec<u8>) {
    // Initialize counters
    let mut zeros = 0_usize;
    let mut bit_multiplier = 1;
    let mut index = 0_usize;

    // Add (bogus) eob symbol to the mtf_index (symbol set)
    mtf_index.push(0);

    // Create the mtf index
    //let mut mtf_index: Vec<u8> = (0_u8..(symbol_set.len()) as u8).map(|n| n).collect();

    // iterate through the input, doing the conversion as we find RUNA/RUNB sequences
    for mtf in data_in {
        // Blow up if the run is too big - this should be more elegant in the future
        if zeros > 2 * 1024 * 1024 {
            error!("Run of zeros exceeded a million - probably input bomb.");
            std::process::exit(100)
        }
        match *mtf {
            // If we found RUNA, do magic to calculate how many zeros we need
            RUNA => {
                zeros += bit_multiplier;
                bit_multiplier *= 2;
            }
            // If we found RUNB, do magic to calculate how many zeros we need
            RUNB => {
                zeros += 2 * bit_multiplier;
                bit_multiplier *= 2;
            }
            // Anything else, first output any pending run of zeros as mtf[0].
            n => {
                if zeros > 0 {
                    for i in index..=index + zeros {
                        out[i] = mtf_index[0]
                    }
                    // Adjust the counters
                    index += zeros;
                    bit_multiplier = 1;
                    zeros = 0;
                }
                // Then output the symbol (one less than n)
                out[index] = mtf_index[n as usize - 1];

                // Increment the index
                index += 1;

                // And adjust the mtf_index for the next symbol
                let sym = mtf_index.remove(n as usize - 1);
                mtf_index.insert(0, sym as u8);
                //Alternately adjust the mtf_index for the next symbol
                // let end = n as usize - 1;

                // for i in 0..end {
                //     unsafe { mtf_index.swap_unchecked(i, end) }
                // }
            }
        }
    }
    // Truncate the vec to the actual data, removing the eob marker.
    out.truncate(index - 1);
}
