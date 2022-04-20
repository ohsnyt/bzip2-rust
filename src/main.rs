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

//use lib::mtf::{mtf_decode, mtf_encode};
use lib::rle1::{rle1_decode, rle1_encode};

fn test() {
    // Prepare to read the data.
    //let filename = "src/highwayman.txt";
    //let filename = "src/idiot2.txt";
    let filename = "src/Peter_Piper 2.txt";
    info!("Working with {}", filename);

    let mut f = File::open(filename).expect("Couldn't find test file.");
    let mut test_data = Vec::new();

    f.read_to_end(&mut test_data)
        .expect("Couldn't read test file.");

    //let test_data = "If Peter Piper picked a peck of pickled peppers, where's the peck of pickled peppers Peter Piper picked?????";
    //let test_data  = "confusedconfusedconfusedconfusedconfusedconfusedconfused1confused2confused3confused4confused5confused6How to encrypt using BWT cipher?";
    info!("Running RLE1 encode and decode");
    //let a = rle1_encode(test_data.as_bytes());
    let a = rle1_encode(&test_data);
    let result = rle1_decode(&a);
    //if std::str::from_utf8(&result).unwrap() == test_data {
    if result == test_data {
        info!("Passed RLE1 encode and decode")
    } else {
        error!("Failed RLE1 encode and decode")
    }
    info!("------");

    info!("Just BWT test");
    let (key, bwt) = lib::bwt::block_sort::block_sort(&test_data, 30);
    let b = lib::bwt_ds::bwt_decode(key as u32, &bwt);
    info!("Key: {}, Input is {} bytes.  BWT produced {} bytes. BWT returned {} bytes.", key, test_data.len(),bwt.len(), b.len());
    if test_data == b {
        info!("Passed BWT encode and decode")
    } else {
        error!("Failed BWT encode and decode")
    };

    info!("RLE with BWT test");
    //let a = rle1_encode(test_data.as_bytes());
    let a = rle1_encode(&test_data);
    let (key, bwt) = lib::bwt::block_sort::block_sort(&a, 10);
    info!("Key: {}, Input is {} bytes. RLE is {} bytes. BWT is {} bytes.", key, test_data.len(), a.len(), bwt.len());
    //info!("{:?}", std::str::from_utf8(&bwt));
    let b = lib::bwt_ds::bwt_decode(key as u32, &bwt);
    if a == b {
        info!("Passed BWT encode and decode")
    } else {
        error!("Failed BWT encode and decode")
    };
    let result = rle1_decode(&b);
    info!("RLE output is {} bytes.", result.len());

    //if std::str::from_utf8(&result).unwrap() == test_data {
    if result == test_data {
        info!("Passed RLE1+BWT encode and decode")
    } else {
        error!("Failed RLE1+BWT encode and decode")
    }
    info!("------");
}
/*
    info!("Adding MTF test");
    let a = rle1_encode(test_data.as_bytes());
    let (key, bwt) = lib::bwt::bwt_encode(&a);
    let (mtf, symbol_map) = mtf_encode(&bwt);
    let index = lib::symbol_map::decode_sym_map(&symbol_map);
    let b = mtf_decode(&mtf, index);
    if bwt == b {
        info!("Passed MTF encode and decode")
    } else {
        error!("Failed MTF encode and decode")
    };
    let c = lib::bwt::bwt_decode(key, &b);
    let result = rle1_decode(&c);
    if std::str::from_utf8(&result).unwrap() == test_data {
        info!("Passed RLE1+BTW+MTF encode and decode")
    } else {
        error!("Failed RLE1+BTW+MTF encode and decode")
    }
    info!("------");

    info!("Adding RLE2 tests");
    let a = rle1_encode(test_data.as_bytes());
    let (key, bwt) = lib::bwt::bwt_encode(&a);
    let (mtf, symbol_map) = mtf_encode(&bwt);
    let index = lib::symbol_map::decode_sym_map(&symbol_map);
    let b = mtf_decode(&mtf, index);
    let (out, _freq_out, _eob) = lib::rle2::rle2_encode(&b);
    let d = lib::rle2::rle2_decode(&out);
    if d == b {
        info!("Passed RLE2 encode and decode")
    } else {
        error!("Failed RLE2 encode and decode")
    };
    let c = lib::bwt::bwt_decode(key, &b);
    let result = rle1_decode(&c);
    if std::str::from_utf8(&result).unwrap() == test_data {
        info!("Passed RLE1+BTW+MTF encode and decode")
    } else {
        error!("Failed RLE1+BTW+MTF encode and decode")
    }
    info!("------");
} */
