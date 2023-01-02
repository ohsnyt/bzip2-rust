//Enable more cargo lint tests
//#![feature(slice_swap_unchecked)]
#![warn(rust_2018_idioms)]
//#![allow(unused)]
#![warn(clippy::disallowed_types)]
//#[global_allocator]
//static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
#![allow(special_module_name)]
mod bitstream;
mod bwt_ribzip;
mod compression;
mod huffman_coding;
mod julian;
mod snyder;
mod tools;

use std::time::{Duration, Instant};

use compression::compress::*;
use compression::decompress::decompress;
use tools::cli::{init_bz_opts, BzOpts, Mode};

use log::{info, warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};

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

    let mut options = BzOpts::new();
    init_bz_opts(&mut options);
    timer.mark("cli");

    //----- Figure how what we need to do
    let result = match options.op_mode {
        Mode::Zip => compress(&mut options, &mut timer),
        Mode::Unzip => decompress(&options, &mut timer),
    };

    timer.mark("misc");
    println!();
    println!("CLI\t\t{:?}", timer.cli);
    println!("BWT\t\t{:?}", timer.bwt);
    println!("Huffman:\t{:?}", timer.huffman);
    println!("MTF:\t\t{:?}", timer.mtf);
    println!("CRCs:\t\t{:?}", timer.crcs);
    println!("RLE:\t\t{:?}", timer.rle);
    println!("RLE1:\t\t{:?}", timer.rle1);
    println!("Setup:\t\t{:?}", timer.setup);
    println!("Cleanup:\t{:?}", timer.cleanup);
    println!("Total:\t\t{:?}\n", timer.total);
    println!(
        "Missing: {:?}",
        timer.total
            - (timer.cli
                + timer.bwt
                + timer.huffman
                + timer.crcs
                + timer.rle1
                + timer.rle
                + timer.setup
                + timer.cleanup)
    );

    // Print out the results
    println!("\nTimer results table:");
    println!(
        "{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
        timer.cli,
        timer.bwt,
        timer.huffman,
        timer.rle,
        timer.mtf,
        timer.rle1,
        timer.setup,
        timer.cleanup,
        timer.total
    );

    info!("Done.\n");
    result
}
