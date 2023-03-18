//Enable more cargo lint tests
#![warn(rust_2018_idioms)]
#![warn(clippy::disallowed_types)]
mod bitstream;
mod bwt_algorithms;
mod compression;
mod huffman_coding;
mod tools;

use crate::tools::cli::bzopts_init;
use compression::{compress::compress, decompress::decompress};
use log::{info, warn, LevelFilter};
use simplelog::{Config, TermLogger, TerminalMode};
use tools::cli::Mode;

fn main() -> Result<(), std::io::Error> {
    // Available log levels are Error, Warn, Info, Debug, Trace
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Stdout,
        simplelog::ColorChoice::AlwaysAnsi,
    )
    .unwrap();

    // Get the options
    let mut options = bzopts_init();

    //----- Figure how what we need to do and go do it
    let result = match options.op_mode {
        Mode::Zip => compress(&mut options),
        Mode::Unzip => decompress(&options),
        Mode::Test => Ok(()),
    };

    info!("Done.\n");
    result
}
