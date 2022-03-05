use super::{options::BzOpts, data_in};
use super::options::Status;

/*
    CHANGE THIS SO INPUT IS PASSED HERE IN BLOCK SIZE PEICES
    THIS NEED TO "RETURN" BLOCK SIZED COMPRESSED PIECES WITH CRCS, OF COURSE.
    THE FINAL FLUSH NEEDS TO RETURN THE STREAM CRC AND FOOTER
*/


/// These are the steps necessary to compress. Input file defined in options.
pub(crate) fn compress(opts: &BzOpts ){

     // Initialize stuff to read the file
    let mut reader = match data_in::init(&opts) {
        Err(_) => {opts.status = Status::NoData; return},
        Ok(reader) => reader,
    };
    // Prepare to write the data. Do this first because we may need to loop and write data multiple times.
    let mut fname = opts.file.as_ref().unwrap().clone();
    fname.push_str(".bz2");
    let mut f_out =
        File::create(&fname).expect(&format!("Unable to create compressed file {}", &fname));

    // Create the BitWriter to write the compressed bit stream output
    let mut bw = BitWriter::new();

    // Put the header onto the bit stream
    bw.out8('B' as u8);
    bw.out8('Z' as u8);
    bw.out8('h' as u8);
    bw.out8(opts.block_size + 0x30);

    //----- Loop through blocks of data and process it.
    'block: loop {
        report(&opts, opts::Verbosity::Chatty, "Starting loop");
        // Try to get a block of data
        let mut crc = 0;
        let next_block = match reader.read() {
            Some(data) => {
                crc = do_crc(&data);
                report(
                    &opts,
                    opts::Verbosity::Chatty,
                    format!("Block length is {} bytes. CRC is {:08x}", data.len(), crc),
                );
                data
            }
            None => break 'block,
        };

        // Now that we have the data, do the first RLE and the BTW.
        let data = rle1_encode(&next_block);
        let (key, mut data) = bwt_encode(&data);
        for b in &data {
            print!("{}", *b as char);
        }
        //println!("");
        //println!("Key is {}", key);

        // Now that we have the key, we can write the block header: Six bytes of magic,
        //   4 bytes of crc data, 1 bit for Randomized flag, and 3 bytes for the 24bit BWT key
        bw.out24(0x18_314159); // magic bits  1-24
        bw.out24(0x18_265359); // magic bits 25-48
        bw.out32(crc); // crc
        bw.out24(0x01_000000); // One zero bit
        bw.out24(0x18_000000 | key); // and 24 bit key

        // And send the BTW data off for the MTF transform...
        //  MTF also returns the symbol map that we need for decompression.
        let (mdata, symbol_map) = mtf_encode(&data);
        // ...followed by the RLE2 transform. These two may later be combined.
        let buf = rle2_encode(&mdata);

        // Now for the compression - the Huffman encoding
        let result = huf_encode(&buf, &mut bw, symbol_map);
        //println!("Result of Huffman encoding is: {:?}", result);

        bw.out32(crc);
        bw.flush();

        // write out the data
        f_out
            .write_all(&bw.output)
            .expect(&format!("Unable to write compressed file {}", &fname));
        report(&opts, opts::Verbosity::Chatty, "BOGUS:Wrote a block");
    }
    //all done. Rust closes the file.
    //println!("Wrote out {}.", fname);
    
}

