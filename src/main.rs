//Enable more cargo lint tests
#![warn(rust_2018_idioms)]
//#![warn(missing_docs)]
//#![warn(missing_debug_implementations)]
//#![allow(unused_variables)]
use std::io;

mod lib;
use lib::options::BzOpts;
use lib::compress::compress;
use lib::decompress::decompress;

use log::{LevelFilter, warn};
use simplelog::{TermLogger, TerminalMode, Config };


fn main() -> io::Result<()> {
    // Available log levels are Error, Warn, Info, Debug, Trace
    TermLogger::init(
        LevelFilter::Trace,
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
        lib::options::Mode::Test => Ok(()),
    }
}

/*-------------------------------------------------------------*/
/*--- public structs, enums and functions for the library   ---*/

