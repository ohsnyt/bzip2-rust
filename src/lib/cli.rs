//use super::options::{BzOpts, Mode, Output, WorkFactor};
use clap::Parser;
use log::info;
use log::warn;
use std::{fmt::Display, fmt::Formatter};

/// Define the alternate compression algorithms
#[derive(Debug, PartialEq, Eq, clap::Subcommand)]
pub enum Algorithms {
    /// Use original Bzip2 Burrow Wheeler Transform algorithm when compressing
    Julian,
    /// Use SAIS based Burrow Wheeler Transform algorithm when compressing
    Sais,
    /// Use simple Burrow Wheeler Transform algorithm when compressing
    Simple,
    // Parallel - uses custom BWT sorting alorithm with Rayon when compressing
    Parallel,
}
/// Define three operational modes
#[derive(Debug)]
pub enum Mode {
    Zip,
    Unzip,
    Test,
}
impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
/// Define the two output channels
pub enum Output {
    File,
    Stdout,
}
impl Display for Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Defines a "fallback" mode for worst case data - may be depricated
#[derive(Debug)]
pub enum WorkFactor {
    Normal = 30,
    Fallback = 1,
}

/// Define all user settable options to control program behavior
#[derive(Debug)]
pub struct BzOpts {
    /// Optional name of file to read for input
    pub file: Option<String>,
    /// Maximum input block size to process during each loop
    pub block_size: u8,
    /// User feedback level setting
    pub op_mode: Mode,
    /// Don't remove input files after processing
    pub keep_input_files: bool,
    /// Optional setting used for oddly constructed data - may be depricated
    pub work_factor: WorkFactor,
    /// Silently overwrite existing files with the same name
    pub force_overwrite: bool,
    /// Location where output is sent
    pub output: Output,
    /// Current status of progress - not yet used
    pub status: Status,
    /// Algorithm used
    pub algorithm: Algorithms,
    /// Iterations used to test/optimize small block compression
    pub iterations: usize,
}

impl BzOpts {
    /// Set default parameters on program start
    pub fn new() -> Self {
        Self {
            file: None,
            block_size: 9,
            op_mode: Mode::Test,
            keep_input_files: false,
            work_factor: WorkFactor::Normal,
            force_overwrite: false,
            output: Output::File,
            status: Status::Init,
            algorithm: Algorithms::Julian,
            iterations: 4,
        }
    }
}

/// NOT YET IMPLEMENTED. Used during the library mode by the calling program.
#[derive(Debug)]
pub enum Status {
    Init,
    NoData,
}

/// Command Line Interpretation - uses external CLAP crate.
#[derive(Parser, Debug)]
#[clap(
    author = "    David M. Snyder <david.snyder@stillwaiting.org>",
    version = "version 0.3.0",
    about = "A Rust implementation of Bzip2",
    long_about = "
    Bzip2 was developed by Julian Seward. The algorithm is based on Huffman encoding of data.
    This version retains the features (and quirks) of the original, but adds some options
    to allow alternate encoding implementations as well as a few tweaks to test/alter 
    the encoding.
    
    It is done in the spirit of learning, both learning Rust and learning compression techniques."
)]
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

    /// Sets verbosity. -v1 shows very little, -v4 is chatty
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

    /// Sort algorithm choice
    #[clap(subcommand)]
    algorithm: Option<Algorithms>,

    /// Compression block iterations. -i4 is the only value used in the original algorithm.
    #[clap(short = 'i', long, default_value_t = 4)]
    iterations: usize,
}

/// Official license statement for Bzip2
fn license() -> String {
    "
bzip2, a block-sorting file compressor.
Copyright (C) 1996-2010 by Julian Seward; 2010-2021 by various; 2022 by David Snyder.
 
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

/// Put command line information from CLAP into our internal structure.
/// NOTE: refactoring may find a way to avoid this step.
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

    // NOTE: This overwrites the best and small flags!
    if args.block_size.is_some() {
        bz_opts.block_size = args.block_size.unwrap()
    };

    if args.license {
        info!("{}", license())
    };

    bz_opts.iterations = args.iterations;

    bz_opts.algorithm = args.algorithm.unwrap_or(Algorithms::Julian);

    // Below we report initialization status to the user
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
    if bz_opts.iterations != 4 {
        info!("Iterations set to {}", bz_opts.iterations)
    };
    info!("---- Bzip2 Initialization End ----\n");
}
