use log::{debug, error, info, warn};

use crate::lib::{
    bwt_decode_2x1::bwt_decode_2x1,
    crc::{do_crc, do_stream_crc},
};
use rustc_hash::FxHashMap;

use super::{
    bitty::BitReader,
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
    time::{Duration, Instant},
};

const BUFFER_SIZE: usize = 50000;
const EOF_MESSAGE: &str = "Unexpected End Of File";

struct Timer {
    setup: Duration,
    huffman: Duration,
    rle2: Duration,
    mtf: Duration,
    btw: Duration,
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
            rle2: Duration::new(0, 0),
            mtf: Duration::new(0, 0),
            btw: Duration::new(0, 0),
            rle1: Duration::new(0, 0),
            cleanup: Duration::new(0, 0),
            total: Duration::new(0, 0),
            time: Instant::now(),
        }
    }
    fn mark(&mut self, item: &str) {
        let dur = self.time.elapsed();
        self.time = Instant::now();
        match item {
            "setup" => {
                self.setup += dur;
                self.total += dur
            }
            "huffman" => {
                self.huffman += dur;
                self.total += dur
            }
            "rle2" => {
                self.rle2 += dur;
                self.total += dur
            }
            "mtf" => {
                self.mtf += dur;
                self.total += dur
            }
            "btw" => {
                self.btw += dur;
                self.total += dur
            }
            "rle1" => {
                self.rle1 += dur;
                self.total += dur
            }
            _ => {
                self.cleanup += dur;
                self.total += dur
            }
        }
    }
}

/// Decompress the file given in the command line
pub(crate) fn decompress(opts: &BzOpts) -> io::Result<()> {
    // TESTING: Mark time
    let mut timer = Timer::new();

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
            if header_footer == vec![0x17, 0x72, 0x45, 0x38, 0x50, 0x90] {
                break 'block;
            }
            // Then create an error if this is not a block header
            if header_footer != vec![0x31_u8, 0x41, 0x59, 0x26, 0x53, 0x59] {
                return Err(Error::new(io::ErrorKind::Other, "Invalid block footer"));
            }
        };
        info!("Found a valid header for block {}.", block_counter);

        // Get crc
        let block_crc = br.bint(32).expect(EOF_MESSAGE);
        info!("CRC is {}.", block_crc);

        // Get randomize bit - should almost always be zero
        let rand = br.bit().expect(EOF_MESSAGE); // Get the randomized flag
        debug!("Randomized is set to {}.", rand);

        // Get key (origin pointer)
        let key = br.bint(24).expect(EOF_MESSAGE);
        debug!("Found BWTransform key ({})", key);
        if key > block_size as u32 * 100000 + 10 {
            return Err(Error::new(io::ErrorKind::Other, "Invalid key pointer"));
        }
        info!("Key is {}.", key);

        // Get the symbol set (dropping the temporary vec used to grab the data)
        let symbol_set: Vec<u8>;
        {
            // First get the map "index" and save it as the first entry in the map
            let mut sym_map: Vec<u16> = vec![br.bint(16).expect(EOF_MESSAGE) as u16];
            // Then get each 16-symbol map as indicated by the set bits in the "index"
            for _ in 0..sym_map[0].count_ones() {
                sym_map.push(br.bint(16).expect(EOF_MESSAGE) as u16);
            }

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
        let mut table_map = Vec::with_capacity(selector_count as usize);
        let mut group: u8 = 0;
        for _ in 0..selector_count {
            while br.bit().expect(EOF_MESSAGE) {
                group += 1;
            }
            // Since Julian ignores the error of excessive selector_count, only push maps that can be used
            if selector_count <= block_size as u32 * 100000 / 50 {
                table_map.push(group);
            }
            group = 0;
        }
        // Adjust the selector_count if needed
        if selector_count > block_size as u32 * 100000 / 50 {
            warn!("Found {} selector were reported, but the maximum is {}. Adjust the selector count down.", selector_count, block_size as u32 * 100000 / 50);
            selector_count = block_size as u32 * 100000 / 50;
        }

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
            "Decoded the {} selectors for the {} tables in block {}.",
            selector_count, table_count, block_counter
        );

        // TESTING: Mark time
        timer.mark("setup");

        // Read the Huffman symbol length maps
        let mut maps: Vec<Vec<(u16, u32)>> = Vec::with_capacity(table_count as usize);
        for _ in 0..table_count {
            let mut map: Vec<(u16, u32)> = vec![(0_u16, 0_u32); symbols];
            let mut l: i32 = br.bint(5).expect(EOF_MESSAGE) as i32;

            for symbol in 0..symbols as u16 {
                let mut diff: i32 = 0;
                loop {
                    if br.bit().expect(EOF_MESSAGE) {
                        if br.bit().expect(EOF_MESSAGE) {
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
                    //info!("Sym:{}, len:{}, last_code.0:{}, last_code.1:{}", sym, len, last_code.0, last_code.1);
                    last_code.1 <<= len - last_code.0;
                    last_code.0 = *len;
                }
                hm_vec[idx].insert(len << 24 | last_code.1, *sym);
                last_code.1 += 1;
            }
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

        // Now read the blocks in chunks of 50 symbols with the huffman tree selected by the selector vec
        let mut block_index = 0;
        'data: for selector in table_map {
            let mut chunk_byte_count = 0;
            let mut bit_count: u32 = 0;
            let mut bits = 0;
            // last symbol in the symbol map marks the end of block (eob)
            // NOTE: IT *might* BE FASTER TO DO A VEC LOOKUP FOR ITEMS OF THE MINIMUM LENGTH
            let eob = (hm_vec[selector].len() - 1) as u16;

            // Loop through the data in 50 byte groups trying to find valid symbols in the bit stream
            while chunk_byte_count < 50 {
                // Left shift any bits thereby adding in one more bit
                bits <<= 1;
                // Set this new bit if needed
                if br.bit().expect(EOF_MESSAGE) {
                    bits |= 1;
                }
                // Update how many bits we have now
                bit_count += 1;
                // Check if we have found a valid symbol code yet (and if not, loop again)
                // IS HASHMAP THE FASTEST WAY TO DO THIS? PERHAPS USE a double vec lookup -
                //   the first with the byte count and the second with the bits???
                if let Some(sym) = hm_vec[selector].get(&(bit_count << 24 | bits)) {
                    // If so, push the symbol out
                    // If we found a valid symbol, push it out.
                    out[block_index] = *sym;
                    // And update the index
                    block_index += 1;

                    // Check if we have reached the end of block
                    if sym != &eob {
                        // If not, reset variables for the next byte
                        bits = 0;
                        bit_count = 0;
                        chunk_byte_count += 1;
                    } else {
                        // Check if we are at the end of the block too early
                        if block_index / 50 < selector_count as usize - 1 {
                            error!("Found EOB before working through all selectors. (Chunk {} instead of {}.)", block_index/50, selector_count)
                        }
                        // Adjust the vec length to the block_index
                        out.truncate(block_index);
                        break 'data;
                    }
                }
            }
        }

        // TESTING: Mark time
        timer.mark("huffman");

        // Undo the RLE2, converting to u8 in the process
        let rle2_v = rle2_decode(&out);

        // TESTING: Mark time
        timer.mark("rle2");

        // Undo the MTF.
        let mtf_v = mtf_decode(&rle2_v, symbol_set.clone());

        // TESTING: Mark time
        timer.mark("mtf");

        /*
        // Undo the MTF - Original version
        let start = Instant::now();
        let bwt_v3 = crate::lib::bwt_ds::bwt_decode_orig(key, &mtf_v);
        warn!("BWTransform original: {:?}", start.elapsed());

        // Undo the MTF - MTL version
        let start = Instant::now();
        let bwt_v4 = crate::lib::bwt_ds::bwt_decode_mtl(key, &mtf_v);
        warn!("BWTransform MLT: {:?}", start.elapsed());

        */
        // Undo the BWTransform
        let start = Instant::now();
        let bwt_v = crate::lib::bwt_ds::bwt_decode(key, &mtf_v); //, &symbol_set);

        // TESTING: Mark time
        timer.mark("btw");

        //println!("New Peter: {}", std::str::from_utf8(&bwt_v[0..26]).unwrap());

        // test
        //let start = Instant::now();
        //let bwt_v2 = bwt_decode_2x1(key, &mtf_v);
        //warn!("BWTransform 2x1: {:?}", start.elapsed());

        // Undo the initial RLE1
        let rle1_v = rle1_decode(&bwt_v);

        // TESTING: Mark time
        timer.mark("rle1");

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
    }

    let final_crc = br.bint(32).expect(EOF_MESSAGE);
    if final_crc == stream_crc {
        info!("Stream CRCs matched: {}.", final_crc);
    } else {
        error!(
            "Stream CRC failed!!! Found {} looking for {}. (Data may be corrupt.)",
            stream_crc, final_crc
        );
    }

    info!("Wrote the decompressed file.\n");

    // TESTING: Mark time
    timer.mark("total");
    println!("--------------------------");
    println!("setup\t{:?}", timer.setup);
    println!("huffman\t{:?}", timer.huffman);
    println!("rle2\t{:?}", timer.rle2);
    println!("mtf\t{:?}", timer.mtf);
    println!("btw\t{:?}", timer.btw);
    println!("rle1\t{:?}", timer.rle1);
    println!("cleanup\t{:?}", timer.cleanup);
    println!("total\t{:?}", timer.total);
    println!("--------------------------");

    Result::Ok(())
}
