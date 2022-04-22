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
        LevelFilter::Debug,
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
use lib::rle1::{rle1_decode, rle1_encode};

use crate::lib::mtf::{mtf_decode, mtf_encode};

fn test() {
    // Prepare to read the data.
    //let filename = "src/highwayman.txt";
    let filename = "/Users/david/Documents/bzip2/bzip2/tests/sample1.ref";
    //let filename = "src/Peter_Piper 2.txt";
    info!("Working with {}", filename);

    let mut f_input = File::open(filename).expect("Couldn't find test file.");
    let mut test_data = Vec::new();

    f_input.read_to_end(&mut test_data)
        .expect("Couldn't read test file.");

    //let test_data = "If Peter Piper picked rle_data peck of pickled peppers, where's the peck of pickled peppers Peter Piper picked?????";
    //let test_data  = "confusedconfusedconfusedconfusedconfusedconfusedconfused1confused2confused3confused4confused5confused6How to encrypt using BWT cipher?";
    info!("Running RLE1 encode and decode");
    //let rle_data = rle1_encode(test_data.as_bytes());
    let rle_data = rle1_encode(&test_data);
    let result = rle1_decode(&rle_data);
    //if std::str::from_utf8(&result).unwrap() == test_data {
    if result == test_data {
        info!("Passed RLE1 encode and decode")
    } else {
        error!("Failed RLE1 encode and decode")
    }
    info!("------");

    info!("Just BWT test");
    let (key, bwt) = lib::bwt::block_sort::block_sort(&test_data, 30);
    let ds_bwt = lib::bwt_ds::bwt_decode(key as u32, &bwt);
    info!(
        "Key: {}, Input is {} bytes.  BWT produced {} bytes. BWT returned {} bytes.",
        key,
        test_data.len(),
        bwt.len(),
        ds_bwt.len()
    );
    if test_data == ds_bwt {
        info!("Passed BWT encode and decode")
    } else {
        error!("Failed BWT encode and decode")
    };

    info!("RLE with BWT test");
    //let rle_data = rle1_encode(test_data.as_bytes());
    let rle_data = rle1_encode(&test_data);
    let (key, bwt) = lib::bwt::block_sort::block_sort(&rle_data, 10);
    info!(
        "Key: {}, Input is {} bytes. RLE is {} bytes. BWT is {} bytes.",
        key,
        test_data.len(),
        rle_data.len(),
        bwt.len()
    );
    //info!("{:?}", std::str::from_utf8(&bwt));
    let ds_bwt = lib::bwt_ds::bwt_decode(key as u32, &bwt);
    if rle_data == ds_bwt {
        info!("Passed BWT encode and decode")
    } else {
        error!("Failed BWT encode and decode")
    };
    let result = rle1_decode(&ds_bwt);
    info!("RLE output is {} bytes.", result.len());

    //if std::str::from_utf8(&result).unwrap() == test_data {
    if result == test_data {
        info!("Passed RLE1+BWT encode and decode")
    } else {
        error!("Failed RLE1+BWT encode and decode")
    }
    info!("------");

    info!("Adding MTF test");
    let rle_data = rle1_encode(&test_data);
    let (key, bwt) = lib::bwt::block_sort::block_sort(&rle_data, 30);
    let (mtf, symbol_map) = mtf_encode(&bwt);
    let index = lib::symbol_map::decode_sym_map(&symbol_map);
    let ds_bwt = mtf_decode(&mtf, index);
    if bwt == ds_bwt {
        info!("Passed MTF encode and decode")
    } else {
        error!("Failed MTF encode and decode")
    };
    let c = bwt_decode(key as u32, &ds_bwt);
    let result = rle1_decode(&c);
    if result == test_data {
        info!("Passed RLE1+BTW+MTF encode and decode")
    } else {
        error!("Failed RLE1+BTW+MTF encode and decode")
    }
    info!("------");

    info!("Adding RLE2 tests");
    let rle_data = rle1_encode(&test_data);
    let (key, bwt) = lib::bwt::block_sort::block_sort(&rle_data, 30);
    let (mtf, symbol_map) = mtf_encode(&bwt);
    let index = lib::symbol_map::decode_sym_map(&symbol_map);
    let ds_bwt = mtf_decode(&mtf, index);
    let (out, _freq_out, _eob) = lib::rle2::rle2_encode(&ds_bwt);
    let rle_data2 = lib::rle2::rle2_decode(&out);
    if rle_data2 == ds_bwt {
        info!("Passed RLE2 encode and decode")
    } else {
        error!("Failed RLE2 encode and decode")
    };
    let c = bwt_decode(key as u32, &ds_bwt);
    let result = rle1_decode(&c);
    if result == test_data {
        info!("Passed RLE1+BTW+MTF encode and decode")
    } else {
        error!("Failed RLE1+BTW+MTF encode and decode")
    }
    info!("------");
}
