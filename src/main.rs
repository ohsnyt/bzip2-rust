//Enable more cargo lint tests
#![warn(rust_2018_idioms)]
#![warn(clippy::disallowed_types)]
mod bitstream;
mod compression;
mod huffman_coding;
mod snyder;
mod tools;

use std::{
    fs::{self, File},
    io::{Read, Write},
};

use compression::compress::*;
use compression::decompress::decompress;
use tools::cli::Mode;

use log::{info, warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};

use crate::tools::cli::bzopts_init;

fn main() -> Result<(), std::io::Error> {
    // Available log levels are Error, Warn, Info, Debug, Trace
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Stdout,
        simplelog::ColorChoice::AlwaysAnsi,
    )
    .unwrap();

    let mut options = bzopts_init();
    // timer.mark("cli");

    // If we are debugging, do that instead of normal operations
    if options.debug {
        debug(&mut options);
        // quit the program
        return Ok(());
    }

    //----- Figure how what we need to do and go do it
    let result = match options.op_mode {
        Mode::Zip => compress(&mut options),
        Mode::Unzip => decompress(&options),
        Mode::Test => Ok(()),
    };

    info!("Done.\n");
    result
}

/// Create a known problem test file and extend it a byte at a time until compression fails.
fn debug(opts: &mut tools::cli::BzOpts) {
    let sourcefile = "src/Peter.txt";
    let testfile = "src/peter_test.txt";
    let compfile = "src/peter_test.tst";

    //let mut fin = File::open(sourcefile).expect("Can't find source file");
    let fin_metadata = fs::metadata(sourcefile).expect("Can't read source metadata");

    for size in 5..fin_metadata.len() as usize {
        println!("----------------------------------");
        println!("Testing size: {}", size);
        println!("----------------------------------");

        let mut buf = vec![0_u8; size];
        let mut fin = File::open(sourcefile).expect("Can't find source file");

        fin.read_exact(&mut buf).expect("Error reading source file");

        {
            // Prepare to write the test file. Do this first because we may need to loop and write data multiple times.
            let fname = testfile;
            let mut f_out = File::create(fname).expect("Error creating test file");
            f_out.write_all(&buf).expect("Can't write test file");

            // Prepare to write the compare file. Do this first because we may need to loop and write data multiple times.
            let fname = compfile;
            let mut f_out = File::create(fname).expect("Error creating compare file");
            f_out.write_all(&buf).expect("Can't write compare file");
        }
        // Set the BzOpts input to the test file
        opts.files = vec![testfile.to_string()];

        // TODO: call compress, then decompress, then compare
        compress(opts).expect("Can't compress test file");
        // Set the BzOpts input to the test.bz file
        let mut test_bz = testfile.to_owned();
        test_bz.push_str(".bz2");
        opts.files = vec![test_bz.clone()];

        // Execute official bzip2 decompress
        let bzd = std::process::Command::new("bzip2")
            .arg("-dkf")
            .arg(test_bz)
            .status()
            .expect("Could not run bzip2 decompress command");

        if !bzd.success() {
            println!("Error occured at byte length: {}!", size);
            panic!("Error occured at byte length: {}!", size)
        }
    }
}
