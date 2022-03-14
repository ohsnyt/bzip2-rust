//Enable more cargo lint tests
#![warn(rust_2018_idioms)]
//#![warn(missing_docs)]
//#![warn(missing_debug_implementations)]
//#![allow(unused_variables)]

mod lib;

fn main() -> io::Result<()> {
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

use std::io;

use crate::lib::options::BzOpts;
use lib::compress::compress;
use lib::decompress::decompress;
