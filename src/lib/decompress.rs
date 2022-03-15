use crate::lib::crc::{do_crc, do_stream_crc};

use super::{
    bitreader::BitReader,
    bwt::bwt_decode,
    mtf::mtf_decode,
    options::{BzOpts, Verbosity::Chatty, Verbosity::Errors, Verbosity::Normal},
    report::report,
    rle1::rle1_decode,
    rle2::rle2_decode,
    symbol_map::decode_sym_map,
};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, Write},
};

pub(crate) fn decompress(opts: &BzOpts) -> io::Result<()> {
    // Initialize stream crc
    let mut stream_crc = 0;

    // Initialize stuff to read the file
    let mut f = "test.txt.bz2".to_string();
    if opts.file.is_none() {
        report(opts, Normal, "Using >test.txt.bz2< as the input file.");
    } else {
        f = opts.file.as_ref().unwrap().to_string()
    }

    let mut br = match File::open(&f) {
        Ok(file) => BitReader::new(file),
        Err(e) => {
            report(opts, Errors, &format!("Cannot read from the file {}", f));
            return Err(e);
        }
    };

    // Look for a valid signature.
    if br.read8plus(24).unwrap() != "BZh".as_bytes() {
        report(opts, Errors, &format!("{} is not a Bzip2 file.", f));
        return Ok(()); // Probably should be an error!!
    }
    report(opts, Chatty, "Found a valid bzip2 signature.");

    // Use the block size to validate the max number of selectors.
    let block_size = br.read8(8).unwrap() - 0x30;
    println!("Block size is {}", block_size);

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
            report(opts, Errors, "Cannot find the start of the first block.");
            return Ok(()); // Probably should be an error!!
        }
        report(
            opts,
            Chatty,
            format!("Found a valid header for block {}.", block_counter),
        );

        // Get crc
        let block_crc = u32::from_be_bytes(br.read8plus(32).unwrap().try_into().unwrap());
        report(opts, Chatty, format!("CRC is {}.", block_crc));

        // Get randomize bit - should almost always be zero
        let rand = br.read8(1).unwrap(); // Get the randomized flag
        report(opts, Chatty, format!("Randomized is set to {}.", rand));

        // Get key (origin pointer)
        let key_vec: Vec<u8> = br.read8plus(24).unwrap();
        let key = (key_vec[0] as u32) << 16 | (key_vec[1] as u32) << 8 | key_vec[2] as u32;
        report(opts, Chatty, format!("Key is {}.", key));
        drop(key_vec);

        // Read the Symbol Map
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
        report(
            opts,
            Chatty,
            format!("Found {} symbols for block {}.", symbols, block_counter),
        );

        // Read NumTrees
        br.ptr("num_tables", 3, "2-6");
        let num_tables = br.read8(3).unwrap();
        report(opts, Chatty, format!("Found {} tables in use.", num_tables));

        // Read NumSels
        br.ptr("num_sels", 15, "0-2000");
        let tmp = br.read8plus(15).unwrap();
        let num_sels: u32 = (tmp[0] as u32) << 7 | tmp[1] as u32;
        report(
            opts,
            Chatty,
            format!(
                "Found {} selectors for block {}. ({} max.)",
                num_sels,
                block_counter,
                block_size as u32 * 100000 / 50
            ),
        );

        // Read Selectors
        br.ptr("table_map", 50, "Variable size");
        let mut table_map = vec![];
        let mut group: u8 = 0;
        for _ in 0..num_sels {
            while br.read8(1).unwrap() == 1 {
                group += 1;
            }
            table_map.push(group);
            group = 0;
        }
        debug_msg(format!("Table map is {:?}", table_map));

        // Get the MTF values for the selectors
        let mut pos = vec![];
        for v in 0..num_tables {
            pos.push(v);
        }
        for selector in table_map.iter_mut() {
            //for i in 0..num_sels as usize {
            let mut v = *selector as usize;
            let tmp = pos[v];
            while v > 0 {
                pos[v] = pos[v - 1];
                v -= 1;
            }
            pos[0] = tmp;
            *selector = tmp;
            //debug_msg(format!("Pushing selector value of {} (should be 0-{})",selector, num_sels));
        }
        report(
            opts,
            Chatty,
            format!("Decoded the selectors for the {} tables.", num_tables),
        );

        // Read the Huffman symbol length maps
        let mut maps: Vec<Vec<(u16, u32)>> = Vec::new();
        let mut diff: i32 = 0;
        for _ in 0..num_tables {
            let mut map: Vec<(u16, u32)> = Vec::new();
            br.ptr("origin", 5, "Base for next huffman tree"); //
            let mut l: i32 = br.read8(5).unwrap() as i32;
            // THIS IS SO STUPID. THIS DOES NOT SET THE FIRST CODE, BUT MERELY SETS THE OFFSET.
            // IT REQUIRES THE NEXT BIT TO BE 0 SO THAT IT CAN BE PUSHED AS A CODE.
            for symbol in 0..symbols as u16 {
                //br.ptr(format!("Next symbol should be here")); // debugging
                loop {
                    let bit = br.read8(1).unwrap();
                    if bit == 0 {
                        //br.ptr(format!("Added index {}, length {}", symbol, (l + diff))); // debugging
                        map.push((symbol, (l + diff) as u32));
                        debug_msg(format!("Pushing {} with width of {}", symbol, (l + diff) as u32));
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
        let mut hm: Vec<HashMap<u32, u16>> = Vec::new(); // will be a vec of the hashmaps
        for i in 0..maps.len() {
            hm.push(HashMap::new());
            // Get the minimum length in use so we can create the "last code" used
            // Lastcode contains the 32bit length and a 32 bit code with the embedded length.
            let mut last_code: (u32, u32) = (maps[i][0].1, 0);
            for (sym, len) in &maps[i] {
                if *len != last_code.0 {
                    last_code.1 <<= len - last_code.0;
                    last_code.0 = *len;
                }
                hm[i].insert(len << 24 | last_code.1, *sym);
                last_code.1 += 1;
            }
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

        for &selector in table_map.iter().take(num_sels as usize) {
            debug_msg(format!("Read symbols for block {}", selector +1));
            let mut block_byte_count = 0;
            let idx = selector as usize;
            let mut bit_count: u32 = 0;
            let mut bits = 0;
            let eob = (maps[idx].len() - 1) as u16; // last symbol in the symbol map is eob
            while block_byte_count < 50 {
                bits <<= 1;
                let bit = br.read8(1).unwrap() as u32;
                bits |= bit;
                bit_count += 1;
                if let Some(sym) = hm[idx].get(&(bit_count << 24 | bits)) {
                    if sym == &eob {
                        // found end of block
                        break;
                    }
                    out.push(*sym);
                    bits = 0;
                    bit_count = 0;
                    block_byte_count += 1;
                }
            }
        }
        // Undo the RLE2
        let rle2_v = rle2_decode(&out);

        // Undo the MTF.
        let mtf_v = mtf_decode(&rle2_v, symbol_set.clone());

        // Undo the BWTransform
        let btw_v = bwt_decode(key, mtf_v); //, &symbol_set);

        // Undo the initial RLE1
        let rle1_v = rle1_decode(&btw_v);

        // Compute the CRC
        let this_crc = do_crc(&rle1_v);
        if block_crc == this_crc {
            report(
                opts,
                Normal,
                format!("Block {} CRCs matched.", block_counter),
            );
        } else {
            report(
                opts,
                Normal,
                format!(
                    "Block {} CRC failed!!! (Continuing to read data.)",
                    block_counter
                ),
            )
        }
        stream_crc = do_stream_crc(stream_crc, this_crc);

        // Done!! Write the data
        let result = f_out.write(&rle1_v);
        report(
            opts,
            Chatty,
            format!("Wrote a block of data with {:?}.", result),
        );
        let result = f_out.flush();
        report(
            opts,
            Chatty,
            format!("Wrote a block of data with {:?}.", result),
        );
    }

    // Now get the block crc and evaluate it later
    //br.ptr("Done with the block. CRC should be next");
    let final_crc = u32::from_be_bytes(br.read8plus(32).unwrap().try_into().unwrap());
    if final_crc == stream_crc {
        report(opts, Normal, "\nStream CRCs matched.");
    } else {
        report(opts, Normal, "Stream CRC failed!!! (Data may be corrupt.)");
    }

    report(opts, Chatty, "Wrote the decompressed file.\n");

    Ok(())
}

fn debug_view<T: Into<u64> + Copy>(name: &str, value: T, width: usize) {
    println!("\n---{}: {:0width$b}\n", name, value.into())
}
fn debug_msg<S: AsRef<str> + std::fmt::Display>(msg: S) {
    println!("\n---{}", msg)
}
