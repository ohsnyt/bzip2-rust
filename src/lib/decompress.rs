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
    symbol_map::decode_sym_map,
};

use std::{
    //collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, Error, Write},
    time::Instant,
};

const BUFFER_SIZE: usize = 50000;

/// Decompress the file given in the command line
pub(crate) fn decompress(opts: &BzOpts) -> io::Result<()> {
    // DEBUG timer
    let start = Instant::now();
    warn!("Time at the start is {:?}.", start.elapsed());

    // Initialize steam CRC value
    let mut stream_crc = 0;

    // Initialize stuff to read the file
    let mut f = "test.txt.bz2".to_string();
    if opts.file.is_none() {
        warn!("Using >test.txt.bz2< as the input file.");
    } else {
        f = opts.file.as_ref().unwrap().to_string()
    }

    let file = File::open(&f)?;
    let metadata = std::fs::metadata(&f)?;

    let mut br = BitReader::new(file, metadata.len() as usize, BUFFER_SIZE);
    debug!("Starting decompression at {}", br.loc());

    // Look for a valid signature. Checking u8s is a bit faster than checking a vec.
    let signature = br.read8plus(24).unwrap();
    if signature != "BZh".as_bytes() {
        info!("Found a valid bzip2 signature.");
    }

    // Use the block size to validate the max number of selectors.
    let block_size_raw = br.read8(8).unwrap();
    if !(0x30..=0x39).contains(&block_size_raw) {
        return Err(Error::new(io::ErrorKind::Other, "Invalid block size"));
    }
    let block_size = block_size_raw - 0x30;

    let mut block_counter = 0;
    'block: loop {
        block_counter += 1;

        // Block header (or footer) should come next.
        let header_footer = br.read8plus(48).unwrap();
        //check for footer first
        if header_footer == vec![0x17, 0x72, 0x45, 0x38, 0x50, 0x90] {
            break 'block;
        }
        if header_footer != vec![0x31_u8, 0x41, 0x59, 0x26, 0x53, 0x59] {
            return Err(Error::new(io::ErrorKind::Other, "Invalid block header"));
        }
        debug!("Found a valid header for block {}.", block_counter);

        // Get crc
        let block_crc = u32::from_be_bytes(br.read8plus(32).unwrap().try_into().unwrap());
        info!("CRC is {}.", block_crc);

        // Get randomize bit - should almost always be zero
        let rand = br.read8(1).unwrap(); // Get the randomized flag
        debug!("Randomized is set to {}.", rand);

        // Get key (origin pointer)
        let debug_loc = br.loc();
        let key_vec: Vec<u8> = br.read8plus(24).unwrap();
        let key = (key_vec[0] as u32) << 16 | (key_vec[1] as u32) << 8 | key_vec[2] as u32;
        debug!("Read key ({})  at {}", key, debug_loc);
        if key > block_size as u32 * 100000 + 10 {
            return Err(Error::new(io::ErrorKind::Other, "Invalid key pointer"));
        }
        info!("Key is {}.", key);
        drop(key_vec);

        // Read the Symbol Map
        let debug_loc = br.loc();
        let sym_map1: u16 = u16::from_be_bytes(br.read8plus(16).unwrap().try_into().unwrap());
        let mut sym_map: Vec<u16> = vec![sym_map1];
        for _ in 0..sym_map1.count_ones() {
            sym_map.push(u16::from_be_bytes(
                br.read8plus(16).unwrap().try_into().unwrap(),
            ));
        }

        // Decode the symbol map
        let symbol_set = decode_sym_map(&sym_map);
        //  and count how many symbols are in the symbol map. The +2 adds in RUNA and RUNB.
        let symbols = symbol_set.len() + 2;
        info!("Found {} symbols for block {}.", symbols, block_counter);
        info!("Read {} symbols at {}", symbols, debug_loc);

        // Read NumTrees
        let debug_loc = br.loc();
        let table_count = br.read8(3).unwrap();
        if !(2..=6).contains(&table_count) {
            return Err(Error::new(io::ErrorKind::Other, "Invalid table count"));
        }
        info!("{} tables in use.", table_count);

        // Read Selector_count (NumSels)
        let debug_loc = br.loc();
        let tmp = br.read8plus(15).unwrap();
        let mut selector_count: u32 = ((tmp[0] as u32) << 8 | tmp[1] as u32) >> 1;
        debug!(
            "Found {} selectors for block {} at {}.",
            selector_count, block_counter, debug_loc,
        );
        warn!("Time ready to read selectors is {:?}.", start.elapsed());

        // Read Selectors based on the actual number of selectors reported
        let mut table_map = Vec::with_capacity(selector_count as usize);
        let mut group: u8 = 0;
        for _ in 0..selector_count {
            while br.read8(1).unwrap() == 1 {
                group += 1;
            }
            // Julian ignores the error of excessive selector_count. Only push maps that can be used
            if selector_count <= block_size as u32 * 100000 / 50 {
                table_map.push(group);
            }
            group = 0;
        }
        // Julian ignores the error of excessive selector_count, and just adjusts the selector_count
        if selector_count > block_size as u32 * 100000 / 50 {
            warn!("Found {} selector were reported, but the maximum is {}. Adjust the selector count down.", selector_count, block_size as u32 * 100000 / 50);
            selector_count = block_size as u32 * 100000 / 50;
        }

        warn!("Time after reading selectors is {:?}.", start.elapsed());

        // Decode selectors from MTF values for the selectors
        // Create an index vec for the number of tables we need
        let mut table_idx: Vec<usize> = (0..table_count as usize).collect();

        // Now undo the move to front for the selectors
        let table_map = table_map
            .iter()
            .fold((Vec::new(), table_idx), |(mut o, mut s), x| {
                o.push(s[*x as usize]);
                let c = s.remove(*x as usize);
                s.insert(0, c);
                (o, s)
            })
            .0;

        info!(
            "Decoded the {} selectors for the {} tables.",
            selector_count, table_count
        );
        warn!("Time after decoding selectors is {:?}.", start.elapsed());

        // Read the Huffman symbol length maps
        let mut maps: Vec<Vec<(u16, u32)>> = Vec::new();
        let mut diff: i32 = 0;
        for _ in 0..table_count {
            let mut map: Vec<(u16, u32)> = Vec::new();
            //let debug_loc = br.loc();
            let mut l: i32 = br.read8(5).unwrap() as i32;

            for symbol in 0..symbols as u16 {
                loop {
                    let bit = br.read8(1).unwrap();
                    if bit == 0 {
                        map.push((symbol, (l + diff) as u32));
                        l += diff;
                        diff = 0;
                        break;
                    } else {
                        let bit = br.read8(1).unwrap();
                        if bit == 0 {
                            diff += 1
                        } else {
                            diff -= 1
                        };
                    }
                }
            }
            //maps must be sorted by length for the next step
            map.sort_by(|a, b| a.1.cmp(&b.1));
            maps.push(map);
        }

        // Build the Huffman decoding maps as a vec of hashmaps. Like before, include the length
        // as part of the hashmap key (8 bits length, 24 bits code). Value is the symbol value.
        let mut hm_vec: Vec<FxHashMap<u32, u16>> = vec![FxHashMap::default(); maps.len()];

        for (idx, map) in maps.iter().enumerate() {
            // Get the minimum length in use so we can create the "last code" used
            // Lastcode contains the 32bit length and a 32 bit code with the embedded length.
            let mut last_code: (u32, u32) = (map[0].1, 0);
            for (sym, len) in map {
                if *len != last_code.0 {
                    last_code.1 <<= len - last_code.0;
                    last_code.0 = *len;
                }
                hm_vec[idx].insert(len << 24 | last_code.1, *sym);
                last_code.1 += 1;
            }
        }
        warn!(
            "Time after building huffman hash maps is {:?}.",
            start.elapsed()
        );

        // Read the data and turn it into a Vec ready for RLE2 decoding
        let mut out = Vec::with_capacity(block_size as usize * 100000);

        // Next comes looping through data and writing it out.
        // First, prepare to write the data.
        let mut fname = opts.file.as_ref().unwrap().clone();
        fname = fname.split(".bz2").map(|s| s.to_string()).collect(); // strip off the .bz2
        fname.push_str(".txt"); // for my testing purposes.
        let mut f_out = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&fname)
            .expect("Can't open file for writing.");

        warn!("---Time so far is {:?}.", start.elapsed());

        // Now read chunks of 50 symbols with the huffman tree selected by the selector vec
        // read the min_len of bits, then a bit at a time more until we get a symbol
        //let mut tmp = br.read8(min_len).unwrap() as u32;
        let mut block_byte_count = 0;

        //for &selector in table_map.iter().take(selector_count as usize) {
        //  (Already solved the excess selctor count above, hence this shorter for statement)
        for selector in table_map {
            let mut chunk_byte_count = 0;
            let mut bit_count: u32 = 0;
            let mut bits = 0;
            // last symbol in the symbol map marks the end of block (eob)
            // NOTE: IT *might* BE FASTER TO DO A VEC LOOKUP FOR ITEMS OF THE MINIMUM LENGTH
            let eob = (hm_vec[selector].len() - 1) as u16;

            // loop through the data in 50 byte groups trying to find valid symbols in the bit stream
            while chunk_byte_count < 50 {
                // make room to get the next bit and tack it on to what we have
                bits <<= 1;
                // get it
                bits |= br.read8(1).unwrap() as u32;
                // update how many bits we have now
                bit_count += 1;
                // check if we have found a valid symbol code yet (and if not, loop again)
                if let Some(sym) = hm_vec[selector].get(&(bit_count << 24 | bits)) {
                    // If so, push the symbol out
                    // HOW CAN WE SPEED THIS UP? BUFFERED WRITING? INDEXED VEC?
                    out.push(*sym);
                    if sym != &eob {
                        // Reset bit counters
                        bits = 0;
                        bit_count = 0;
                        chunk_byte_count += 1;
                        block_byte_count += 1; // for trace debugging
                                               //debug_loc = br.loc();
                    } else {
                        // FOUND EOB
                        if block_byte_count / 50 < selector_count - 1 {
                            error!("Found EOB before working through all selectors. (Chunk {} instead of {}.)", block_byte_count/50, selector_count)
                        }
                        warn!("Time Huffman decoding is done  is {:?}.", start.elapsed());

                        // Undo the RLE2
                        let rle2_v = rle2_decode(&out);
                        warn!("Time RLE2 is done  is {:?}.", start.elapsed());

                        // Undo the MTF.
                        let mtf_v = mtf_decode(&rle2_v, symbol_set.clone());
                        warn!("Time MTF is done  is {:?}.", start.elapsed());

                        // Undo the BWTransform
                        let btw_v = crate::lib::bwt_ds::bwt_decode(key, &mtf_v); //, &symbol_set);
                        warn!("Time BTW is done  is {:?}.", start.elapsed());

                        // Undo the initial RLE1
                        let rle1_v = rle1_decode(&btw_v);
                        warn!("Time RLE1 is done  is {:?}.", start.elapsed());

                        // Compute the CRC
                        let this_block_crc = do_crc(0, &rle1_v);
                        stream_crc = do_stream_crc(stream_crc, this_block_crc);

                        if block_crc == this_block_crc {
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
                        warn!("Time at this point is {:?}.", start.elapsed());

                        // break out of while loop
                        break;
                    }
                }
            }
        }
        warn!("Time at the end is {:?}.", start.elapsed());
    }

    debug!("Looking for final crc at {}", br.loc());
    let final_crc = u32::from_be_bytes(br.read8plus(32).unwrap().try_into().unwrap());
    if final_crc == stream_crc {
        info!("Stream CRCs matched: {}.", final_crc);
    } else {
        error!(
            "Stream CRC failed!!! Found {} looking for {}. (Data may be corrupt.)",
            stream_crc, final_crc
        );
    }

    info!("Wrote the decompressed file.\n");

    Result::Ok(())
}
