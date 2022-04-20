use log::{debug, error, info, trace, warn};

use super::{
    bitreader::BitReader,
    //bwt_ds::bwt_decode,
    //bwt_inverse::inverse_bwt,
    crc::{do_crc, do_stream_crc},
    mtf::mtf_decode,
    options::BzOpts,
    rle1::rle1_decode,
    rle2::rle2_decode,
    symbol_map::decode_sym_map,
};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, Write},
};

/// Decompress the file given in the command line
pub(crate) fn decompress(opts: &BzOpts) -> io::Result<()> {
    // Initialize stream crc
    let mut stream_crc = 0;

    // Initialize stuff to read the file
    let mut f = "test.txt.bz2".to_string();
    if opts.file.is_none() {
        warn!("Using >test.txt.bz2< as the input file.");
    } else {
        f = opts.file.as_ref().unwrap().to_string()
    }

    let mut br = match File::open(&f) {
        Ok(file) => BitReader::new(file),
        Err(e) => {
            error!("Cannot read from the file {}", f);
            return Err(e);
        }
    };
    debug!("Starting decompression at {}", br.loc());

    // Look for a valid signature.
    if br.read8plus(24).unwrap() != "BZh".as_bytes() {
        info!("{} is not a Bzip2 file.", f);
        return Ok(()); // Probably should be an error!!
    }
    info!("Found a valid bzip2 signature.");

    // Use the block size to validate the max number of selectors.
    let block_size = br.read8(8).unwrap() - 0x30;

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
            warn!("Cannot find the start of the first block.");
            return Ok(()); // Probably should be an error!!
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
        //  and count how many symbols are in the symbol map
        let symbols = symbol_set.len() + 2; // I'm not sure +2 is needed. Watch.
        info!("Found {} symbols for block {}.", symbols, block_counter);
        debug!("Read {} symbols at {}", symbols, debug_loc);

        // Read NumTrees
        let debug_loc = br.loc();
        let table_count = br.read8(3).unwrap();
        debug!(
            "table_count is {}, (should be 2-6) at {}",
            table_count, debug_loc
        );
        info!("{} tables in use.", table_count);

        // Read Selector_count (NumSels)
        let debug_loc = br.loc();
        let tmp = br.read8plus(15).unwrap();
        let selector_count: u32 = (tmp[0] as u32) << 7 | tmp[1] as u32;
        trace!("selector_count is {} at {}", selector_count, debug_loc);

        info!(
            "Found {} selectors for block {}. ({} max.)",
            selector_count,
            block_counter,
            block_size as u32 * 100000 / 50
        );

        // Read Selectors
        let debug_loc = br.loc();
        let mut table_map = Vec::with_capacity(selector_count as usize);
        let mut group: u8 = 0;
        for _ in 0..selector_count {
            while br.read8(1).unwrap() == 1 {
                group += 1;
            }
            table_map.push(group);
            group = 0;
        }
        trace!("Read {} selectors at {}", selector_count, debug_loc);
        trace!("Read mtf version of selectors {:?}", table_map);

        // Decode selectors from MTF values for the selectors
        // create an index from 0 to table_count long, incrementing each value
        let mut table_idx = vec![];
        for v in 0..table_count {
            table_idx.push(v);
        }
        // then undo the move to front
        let table_map = table_map
            .iter()
            .fold((Vec::new(), table_idx), |(mut o, mut s), x| {
                o.push(s[*x as usize]);
                let c = s.remove(*x as usize);
                s.insert(0, c);
                (o, s)
            })
            .0;

        trace!("Decoded selector (table) map is {:?}", table_map);
        info!(
            "Decoded the {} selectors for the {} tables.",
            selector_count, table_count
        );

        // Read the Huffman symbol length maps
        let mut maps: Vec<Vec<(u16, u32)>> = Vec::new();
        let mut diff: i32 = 0;
        for _ in 0..table_count {
            let mut map: Vec<(u16, u32)> = Vec::new();
            let debug_loc = br.loc();
            let mut l: i32 = br.read8(5).unwrap() as i32;
            debug!("Read origin (first symbol length) ({}) at {}", l, debug_loc);

            for symbol in 0..symbols as u16 {
                loop {
                    let bit = br.read8(1).unwrap();
                    if bit == 0 {
                        trace!("Added index {}, length {}", symbol, (l + diff));
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
            // pretty print debug info for tables
            //let pretty_map = map.iter().map(|(_, l)| format!("{:0?}", l)).collect::<Vec<String>>();
            //debug!("{:?}", pretty_map);
            //maps must be sorted by length for the next step
            map.sort_by(|a, b| a.1.cmp(&b.1));
            maps.push(map);
        }

        // Build the Huffman decoding maps as a vec of hashmaps. Like before, include the length
        // as part of the hashmap key (8 bits length, 24 bits code). Value is the symbol value.
        let mut hm_vec: Vec<HashMap<u32, u16>> = vec![]; // will be a vec of the hashmaps
        for map in &maps {
            // create a blank hashmap
            let mut hm = HashMap::new();
            // Get the minimum length in use so we can create the "last code" used
            // Lastcode contains the 32bit length and a 32 bit code with the embedded length.
            let mut last_code: (u32, u32) = (map[0].1, 0);
            for (sym, len) in map {
                if *len != last_code.0 {
                    last_code.1 <<= len - last_code.0;
                    last_code.0 = *len;
                }
                hm.insert(len << 24 | last_code.1, *sym);
                last_code.1 += 1;
            }
            hm_vec.push(hm);
        }
        // Read the data and turn it into a Vec ready for RLE2 decoding
        let mut out = vec![];

        // Next comes looping through data and writing it out.
        // First, prepare to write the data.
        let mut fname = opts.file.as_ref().unwrap().clone();
        fname = fname.split(".bz2").map(|s| s.to_string()).collect(); // strip off the .bz2
        fname.push_str(".txt"); // for my testing purposes.
        let mut f_out = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&fname)?;

        // Now read chunks of 50 symbols with the huffman tree selected by the selector vec
        // read the min_len of bits, then a bit at a time more until we get a symbol
        //let mut tmp = br.read8(min_len).unwrap() as u32;
        let mut block_byte_count = 0;

        for &selector in table_map.iter().take(selector_count as usize) {
            trace!(
                "Read 50 byte chunk at {} using table {}",
                br.loc(),
                selector
            );
            let mut chunk_byte_count = 0;
            let idx = selector as usize;
            let mut bit_count: u32 = 0;
            let mut bits = 0;
            // last symbol in the symbol map is eob
            let eob = (hm_vec[idx].len() - 1) as u16;
            let mut debug_loc = br.loc();

            // loop through the data in 50 byte groups
            while chunk_byte_count < 50 {
                bits <<= 1;
                bits |= br.read8(1).unwrap() as u32;
                bit_count += 1;
                // check if we have found a valid symbol code yet
                if let Some(sym) = hm_vec[idx].get(&(bit_count << 24 | bits)) {
                    trace!(
                        "Byte {}, loc: {} found {}, code {:0width$b}",
                        block_byte_count,
                        debug_loc,
                        sym,
                        bits,
                        width = bit_count as usize
                    );
                    // Push the symbol out
                    out.push(*sym);
                    if sym != &eob {
                        // Reset bit counters
                        bits = 0;
                        bit_count = 0;
                        chunk_byte_count += 1;
                        block_byte_count += 1; // for trace debugging
                        debug_loc = br.loc();
                    } else {
                        // FOUND EOB
                        if block_byte_count / 50 < selector_count - 1 {
                            error!("Found EOB before working through all selectors. (Chunk {} instead of {}.)", block_byte_count/50, selector_count)
                        }
                        // Undo the RLE2
                        let rle2_v = rle2_decode(&out);

                        trace!("MTF input is {:?}", std::str::from_utf8(&rle2_v).unwrap());
                        // Undo the MTF.
                        let mtf_v = mtf_decode(&rle2_v, symbol_set.clone());
                        trace!(
                            "Entering BWT with key of {} and data of \n{:?}",
                            key,
                            std::str::from_utf8(&mtf_v)
                        );

                        // Undo the BWTransform
                        let btw_v = crate::lib::bwt_ds::bwt_decode(key, &mtf_v); //, &symbol_set);
                        trace!("Left BWT with \n{:?}", std::str::from_utf8(&btw_v));

                        // Undo the initial RLE1
                        let rle1_v = rle1_decode(&btw_v);
                        trace!("Left RLE1 with \n{:?}", std::str::from_utf8(&rle1_v));

                        // Compute the CRC
                        let this_crc = do_crc(&rle1_v);
                        stream_crc = do_stream_crc(stream_crc, this_crc);
                        if block_crc == this_crc {
                            info!("Block {} CRCs matched.", block_counter);
                        } else {
                            warn!(
                                "Block {} CRC failed!!! Found {} looking for {}. (Continuing...)",
                                block_counter, this_crc, block_crc
                            );
                        }

                        // Done!! Write the data
                        let result = f_out.write(&rle1_v);
                        info!("Wrote a block of data with {} bytes.", result.unwrap());
                        // break out of while loop
                        break;
                    }
                }
            }
        }
    }

    debug!("Looking for final crc at {}", br.loc());
    let final_crc = u32::from_be_bytes(br.read8plus(32).unwrap().try_into().unwrap());
    if final_crc == stream_crc {
        info!("Stream CRCs matched.");
    } else {
        warn!(
            "Stream CRC failed!!! Found {} looking for {}. (Data may be corrupt.)",
            final_crc, stream_crc
        );
    }

    info!("Wrote the decompressed file.\n");

    Ok(())
}
