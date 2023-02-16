//Enable more cargo lint tests
#![warn(rust_2018_idioms)]
#![warn(clippy::disallowed_types)]
mod bitstream;
mod bwt_ribzip;
mod compression;
mod huffman_coding;
mod julian;
mod snyder;
mod tools;

use std::{
    fs::{self, File},
    io::{Read, Write},
    time::{Duration, Instant},
};

use compression::compress::*;
use compression::decompress::decompress;
use tools::cli::Mode;

use log::{info, log_enabled, warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};

use crate::tools::cli::bzopts_init;

pub struct Timer {
    pub cli: Duration,
    pub setup: Duration,
    pub h_bitread: Duration,
    pub huffman: Duration,
    pub mtf: Duration,
    pub rle: Duration,
    pub rle_mtf: Duration,
    pub bwt: Duration,
    pub rle1: Duration,
    pub crcs: Duration,
    pub cleanup: Duration,
    pub total: Duration,
    pub time: Instant,
}
impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
impl Timer {
    pub fn new() -> Self {
        Self {
            cli: Duration::new(0, 0),
            setup: Duration::new(0, 0),
            h_bitread: Duration::new(0, 0),
            huffman: Duration::new(0, 0),
            mtf: Duration::new(0, 0),
            rle: Duration::new(0, 0),
            rle_mtf: Duration::new(0, 0),
            bwt: Duration::new(0, 0),
            rle1: Duration::new(0, 0),
            crcs: Duration::new(0, 0),
            cleanup: Duration::new(0, 0),
            total: Duration::new(0, 0),
            time: Instant::now(),
        }
    }

    pub fn mark(&mut self, area: &str) {
        match area {
            "cli" => {
                self.cli += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
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
            "h_bitread" => {
                self.h_bitread += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
            "mtf" => {
                self.mtf += self.time.elapsed();
                self.total += self.time.elapsed();
                self.time = Instant::now();
            }
            "rle" => {
                self.rle += self.time.elapsed();
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
            "crcs" => {
                self.crcs += self.time.elapsed();
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

fn main() -> Result<(), std::io::Error> {
    // For testing, set up timer
    let mut timer = Timer::new();

    // Available log levels are Error, Warn, Info, Debug, Trace
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Stdout,
        simplelog::ColorChoice::AlwaysAnsi,
    )
    .unwrap();

    let mut options = bzopts_init();
    timer.mark("cli");

    // If we are debugging, do that instead of normal operations
    if options.debug {
        debug(&mut options, &mut timer);
        // quit the program
        return Ok(());
    }

    //----- Figure how what we need to do and go do it
    let result = match options.op_mode {
        Mode::Zip => compress(&mut options, &mut timer),
        Mode::Unzip => decompress(&options, &mut timer),
        Mode::Test => Ok(()),
    };

    if log_enabled!(log::Level::Warn) {
        timer.mark("misc");
        println!("{:>14} Function", "Microseconds");
        println!("{:>14} CLI", timer.cli.as_micros());
        println!("{:>14} Setup", timer.setup.as_micros());
        println!("{:>14} RLE1", timer.rle1.as_micros());
        println!("{:>14} CRCs", timer.crcs.as_micros());
        println!("{:>14} BWT", timer.bwt.as_micros());
        println!("{:>14} RLE2-MTF", timer.rle_mtf.as_micros());
        println!("{:>14} Huffman", timer.huffman.as_micros());
        println!("{:>14} Cleanup", timer.cleanup.as_micros());
        println!("{:>14} Total", timer.total.as_micros());
        println!();
        println!(
            "{:>14} Missing",
            timer.total.as_micros()
                - (timer.cli
                    + timer.setup
                    + timer.rle1
                    + timer.crcs
                    + timer.bwt
                    + timer.rle_mtf
                    + timer.huffman
                    + timer.cleanup)
                    .as_micros()
        );

        // Print out the results
        println!("\nTimer results table:");
        println!(
            "{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
            timer.cli.as_micros(),
            timer.setup.as_micros(),
            timer.rle1.as_micros(),
            timer.crcs.as_micros(),
            timer.bwt.as_micros(),
            timer.rle_mtf.as_micros(),
            timer.huffman.as_micros(),
            timer.cleanup.as_micros(),
            timer.total.as_micros()
        );
    }

    info!("Done.\n");
    result
}

/// Create a known problem test file and extend it a byte at a time until compression fails.
fn debug(opts: &mut tools::cli::BzOpts, timer: &mut Timer) {
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
        compress(opts, timer).expect("Can't compress test file");
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
