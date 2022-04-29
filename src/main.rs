//Enable more cargo lint tests
#![warn(rust_2018_idioms)]
//#![warn(missing_docs)]
//#![warn(missing_debug_implementations)]
//#![allow(unused_variables)]
use std::{
    fs::File,
    io::{self, Read},
};

mod lib;
use lib::compress::compress;
use lib::decompress::decompress;
use lib::options::BzOpts;

use log::{error, info, warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};

fn main() -> io::Result<()> {
    // Available log levels are Error, Warn, Info, Debug, Trace
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        simplelog::ColorChoice::AlwaysAnsi,
    )
    .unwrap();

    let mut options = BzOpts::new();
    lib::cli::init_bz_opts(&mut options);

    //----- Figure how what we need to do
    match options.op_mode {
        lib::options::Mode::Zip => compress(&mut options),
        lib::options::Mode::Unzip => decompress(&options),
        lib::options::Mode::Test => {
            test();
            Ok(())
        }
    }
}

/*-------------------------------------------------------------*/
/*--- public structs, enums and functions for the library   ---*/

use crate::lib::bwt_ds::bwt_decode;
use lib::rle1::rle1_decode;
use lib::rle1::Encode;

use crate::lib::mtf::{mtf_decode, mtf_encode};

fn test() {
    // Prepare to read the data.
    //let filename = "src/highwayman.txt";
    //let filename = "/Users/david/Documents/bzip2/bzip2/tests/sample1.ref";
    //let filename = "src/Peter_Piper 2.txt";
    //let filename = "src/test.txt";
    let filename = "src/idiot.txt";
    info!("Working with {}", filename);

    let mut f_input = File::open(filename).expect("Couldn't find test file.");
    let mut test_data = Vec::new();

    f_input
        .read_to_end(&mut test_data)
        .expect("Couldn't read test file.");

        
        // FOR TESTING
        let start = 475;
        for i in start..10000 {
            let end = i;
            println!("\n\n******************************");
            println!("     Testing {} bytes", end);
            println!("******************************\n");

            // Initialize encoder
        let mut rle = Encode::new();
        // Set block size. A size less than 4 makes no sense.
        let max = end;
        // Initialize a vec to receive the RLE data
        let mut rle_data: Vec<u8> = Vec::with_capacity(max + 19);
        // Loop through the block
        for el in &test_data[0..end] {
            if let (Some(byte), part_b) = rle.next(*el) {
                rle_data.push(byte);
                if part_b.is_some() {
                    rle_data.push(part_b.unwrap())
                }
            }
        }
        // Flush the encoder at the end in case we
        if let Some(byte) = rle.flush() {
            rle_data.push(byte)
        }

        let result = rle1_decode(&rle_data);
        if result == test_data[0..end] {
            info!("Passed RLE1 encode and decode")
        } else {
            error!("Failed RLE1 encode and decode...");
            for i in 0..result.len().min(test_data.len()) {
                if result[i] != test_data[i] {
                    println!("Data wrong at index {}: {}, {}", i, result[i], test_data[i]);
                    println!("{:?}", &result[(i - 2)..(i + 2)]);
                }
            }
            break
        }
        info!("-------------------------------------\n");

        info!("Just BWT test");
        let (key, bwt) = lib::bwt::block_sort::block_sort(&test_data[0..end], 30);
        let ds_bwt = lib::bwt_ds::bwt_decode(key as u32, &bwt);
        info!(
            "Key: {}, Input is {} bytes.  BWT produced {} bytes. BWT returned {} bytes.",
            key,
            end,
            bwt.len(),
            ds_bwt.len()
        );
        if test_data[0..end] == ds_bwt {
            info!("Passed BWT encode and decode")
        } else {
            error!("Failed BWT encode and decode");
            break
        };

        // info!("-------------------------------------------\n");
        // info!("ds BWT test");
        // let (key, bwt) = lib::bwt::block_sort::block_sort(&test_data[0..end], 30);
        // let (key2, bwt2) = lib::bwt_ds::bwt_encode(&test_data[0..end]);
        // info!(
        //     "Key: {}, Input is {} bytes.  BWT produced {} bytes. BWT2 returned {} bytes.",
        //     key,
        //     end,
        //     bwt.len(),
        //     bwt2.len()
        // );
        // if key == key2 as usize {
        //     info!("keys match")
        // } else {
        //     error!("Key 1 {}, key 2 {}", key, &key2)
        // };
        // if bwt == bwt2 {
        //     info!("Both BWT identical")
        // } else {
        //     error!("BWTs differ")
        // };
        // let decode = lib::bwt_ds::bwt_decode(key as u32, &bwt);
        // let decode2 = lib::bwt_ds::bwt_decode(key2 as u32, &bwt2);
        // if decode == test_data[0..end] {
        //     info!("Passed NEW BWT encode and decode")
        // } else {
        //     error!("Failed NEW BWT encode and decode")
        // };
        // if decode2 == test_data[0..end] {
        //     info!("Passed slow BWT2 encode and decode")
        // } else {
        //     error!("Failed slow BWT encode and decode");
        //             return;

        // };
        // info!("-------------------------------------------\n");

        // info!("RLE with BWT test");
        // let (key, bwt) = lib::bwt::block_sort::block_sort(&rle_data, 10);
        // info!(
        //     "Key: {}, Input is {} bytes. RLE is {} bytes. BWT is {} bytes.",
        //     key,
        //     end,
        //     rle_data.len(),
        //     bwt.len()
        // );
        // let ds_bwt = lib::bwt_ds::bwt_decode(key as u32, &bwt);
        // if rle_data == ds_bwt {
        //     info!("Passed BWT encode and decode")
        // } else {
        //     error!("Failed BWT encode and decode")
        // };
        // let result = rle1_decode(&ds_bwt);
        // info!("RLE output is {} bytes.", result.len());

        // if result == test_data[0..end] {
        //     info!("Passed RLE1+BWT encode and decode")
        // } else {
        //     error!("Failed RLE1+BWT encode and decode")
        // }
        //info!("-------------------------------------\n");

        info!("Adding MTF test");
        let (mtf, symbol_map) = mtf_encode(&bwt);
        // let index = lib::symbol_map::decode_sym_map(&symbol_map);
        // let ds_bwt = mtf_decode(&mtf, index);
        // if bwt == ds_bwt {
        //     info!("Passed MTF encode and decode")
        // } else {
        //     error!("Failed MTF encode and decode")
        // };
        // let c = bwt_decode(key as u32, &ds_bwt);
        // let result = rle1_decode(&c);
        // if result == test_data[0..end] {
        //     info!("Passed RLE1+BTW+MTF encode and decode")
        // } else {
        //     error!("Failed RLE1+BTW+MTF encode and decode")
        // }
        //info!("-------------------------------------\n");

        info!("Adding RLE2 tests");
        let index = lib::symbol_map::decode_sym_map(&symbol_map);
        let ds_bwt = mtf_decode(&mtf, index);
        //let (out, _freq_out, _eob) = lib::rle2::rle2_encode(&ds_bwt);
        //let rle_data2 = lib::rle2::rle2_decode(&out);
        // if rle_data2 == ds_bwt {
        //     info!("Passed RLE2 encode and decode")
        // } else {
        //     error!("Failed RLE2 encode and decode")
        // };
        let c = bwt_decode(key as u32, &ds_bwt);
        let result = rle1_decode(&c);
        if result == test_data[0..end] {
            info!("Passed RLE1+BTW+MTF encode and decode")
        } else {
            error!("Failed RLE1+BTW+MTF+RLE2 encode and decode");
            break
        }
        //info!("-------------------------------------\n");
    }
}
