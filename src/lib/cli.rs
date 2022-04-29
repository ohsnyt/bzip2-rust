use super::options::{BzOpts, Mode, Output, WorkFactor};
use clap::Parser;
use log::info;
use log::warn;

/// Command Line Interpretation - uses external CLAP crate.
/// (Define author, version and about here.)
/// Needs to be reworked. This is barely working.
#[derive(Parser, Debug)]
#[clap(
    author = "Micah D. Snyder <zzz@gmail.com>",
    version = "version 2.0",
    about = "A fast, robust compression/decompression tool",
    long_about = None)]
pub struct Args {
    /// Filename of file to process
    #[clap()]
    filename: Option<String>,
    /// Perform compression on the input file
    #[clap(short = 'z', long = "zip")]
    compress: bool,

    /// Perform decompression on the input file
    #[clap(short = 'd', long = "decompress")]
    decompress: bool,

    ///Force overwriting output file
    #[clap(short = 'f', long = "force")]
    force: bool,

    /// Keep input file
    #[clap(short = 'k', long = "keep")]
    keep: bool,

    /// Send output to the terminal
    #[clap(short = 'c', long = "stdout")]
    stdout: bool,

    /// Shift into fallback mode, useful for highly repetitive data
    #[clap(long = "workfactor")]
    workfactor: bool,

    /// Reduce memory requirements
    #[clap(short = 's', long = "small")]
    small: bool,

    /// Test compressed file integrity
    #[clap(short = 't', long = "test")]
    test: bool,

    /// Alias for 100k block mode
    #[clap(long = "fast")]
    fast: bool,

    /// Alias for 900k block mode
    #[clap(long = "best")]
    best: bool,

    /// Sets verbosity. -v shows very little, -vvvv is chatty
    #[clap(short = 'v', default_value_t = 3)]
    v: u8,

    /// Displays version information
    #[clap(short = 'V', long = "version")]

    /// Displays license information
    #[clap(short = 'L', long)]
    license: bool,

    /// 1..9 - Set the block size from 100-900k. 900k is the default
    #[clap()]
    block_size: Option<u8>,
}

/// Official license statement for Bzip2
fn license() -> String {
    "
bzip2, a block-sorting file compressor.
Copyright (C) 1996-2010 by Julian Seward; 2010-2021 by various; 2022 by Micah D. Snyder.
 
This program is free software; you can redistribute it and/or modify
it under the terms set out in the LICENSE file, which is included
in the bzip2-1.0.6 source distribution.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
LICENSE file for more details.

"
    .to_string()
}

/// Copy command line stuff from that module's style into our internal structure
/// refactoring may find a way to avoid this step (then lot status to user)
pub fn init_bz_opts(bz_opts: &mut BzOpts) {
    let args = Args::parse();

    if args.filename.is_some() {
        bz_opts.file = Some(args.filename.as_ref().unwrap().to_string())
    };
    if args.compress {
        bz_opts.op_mode = Mode::Zip
    };
    if args.decompress {
        bz_opts.op_mode = Mode::Unzip
    };
    bz_opts.force_overwrite = args.force;
    bz_opts.keep_input_files = args.keep;
    if args.stdout {
        bz_opts.output = Output::Stdout
    };
    if args.workfactor {
        bz_opts.work_factor = WorkFactor::Fallback
    };
    if args.small {
        bz_opts.block_size = 2
    };
    if args.test {
        bz_opts.op_mode = Mode::Test
    };
    if args.fast {
        bz_opts.block_size = 2
    };
    // May also need to set something for decompression algorithm
    if args.best {
        bz_opts.block_size = 9
    };
    // Set the log level
    match args.v {
        1 => log::set_max_level(log::LevelFilter::Off),
        2 => log::set_max_level(log::LevelFilter::Error),
        3 => log::set_max_level(log::LevelFilter::Info),
        _ => log::set_max_level(log::LevelFilter::Debug),
    };
    if args.block_size.is_some() {
        bz_opts.block_size = args.block_size.unwrap()
    };
    if args.license {
        info!("{}", license())
    };

    // Below we lot initialization status to the user
    info!("---- Bzip2 Initialization Start ----",);
    info!("Verbosity set to {}", log::max_level());
    info!("Operational mode set to {}", bz_opts.op_mode);
    match &bz_opts.file {
        Some(s) => info!("Getting input from the file {}", s),
        None => warn!("Sending output to stdout"),
    }
    info!("Block size set to {}", bz_opts.block_size);
    if bz_opts.force_overwrite {
        info!("Forcing file overwriting")
    };
    if bz_opts.keep_input_files {
        info!("Keeping input files")
    };
    info!("---- Bzip2 Initialization End ----\n");
}
