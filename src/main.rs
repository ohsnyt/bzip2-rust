//Enable more cargo lint tests
//#![feature(slice_swap_unchecked)]
#![warn(rust_2018_idioms)]
//#![allow(unused)]
#![warn(clippy::disallowed_types)]
//#[global_allocator]
//static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
#![allow(clippy::special_module_name)]
mod bitstream;
mod bwt_ribzip;
mod compression;
mod tools;
mod julian;
mod snyder;
mod huffman_coding;

use compression::compress::*;
use compression::decompress::decompress;
use tools::cli::{BzOpts, Mode, init_bz_opts};



use log::{warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};

fn main() -> Result<(), std::io::Error> {
    // Available log levels are Error, Warn, Info, Debug, Trace
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Stdout,
        simplelog::ColorChoice::AlwaysAnsi,
    )
    .unwrap();

    let mut options = BzOpts::new();
    init_bz_opts(&mut options);

    //----- Figure how what we need to do
    match options.op_mode {
        Mode::Zip => compress(&mut options),
        Mode::Unzip => decompress(&options),
        Mode::Test => {
            //test();
            Result::Ok(())
        }
    }
}
