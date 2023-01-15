//use super::options::{BzOpts, Mode, Output, WorkFactor};
//use clap::Parser;
//use log::info;
//use log::warn;
use std::{fmt::Display, fmt::Formatter};

use std::process::exit;

/// Verbosity of user information
#[derive(Debug)]
pub enum Verbosity {
    Quiet,
    Errors,
    Warnings,
    Info,
    Debug,
    Trace,
}
#[derive(Debug)]

/// Zip, Unzip, Test
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

/// NOT YET IMPLEMENTED. Used during the library mode by the calling program.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Status {
    Init,
    NoData,
}

#[derive(Debug)]
pub struct BzOpts {
    /// Algorithm used
    pub algorithm: Algorithms,
    /// Maximum input block size to process during each loop
    pub block_size: usize,
    /// Vec of names of files to read for input
    pub files: Vec<String>,
    /// Silently overwrite existing files with the same name
    pub force_overwrite: bool,
    /// Don't remove input files after processing
    pub keep_input_files: bool,
    /// Iterations used to test/optimize small block compression
    pub iterations: usize,
    /// Compress/Decompress/Test
    pub op_mode: Mode,
    /// Location where output is sent
    pub output: Output,
    /// Small memory footprint requested
    pub small: bool,
    /// Current status of progress - not yet used
    pub status: Status,
    /// Verbosity of user information
    pub verbose: Verbosity,
    /// Optional setting used for oddly constructed data - may be depricated
    pub work_factor: usize,
}

impl BzOpts {
    pub fn new() -> Self {
        Self {
            algorithm: Algorithms::Simple,
            block_size: 9,
            files: vec![],
            force_overwrite: false,
            keep_input_files: false,
            iterations: 4,
            op_mode: Mode::Zip,
            output: Output::File,
            small: false,
            status: Status::Init,
            verbose: Verbosity::Errors,
            work_factor: 30,
        }
    }
}

impl Default for BzOpts {
    fn default() -> Self {
        Self::new()
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn bzopts_init() -> BzOpts {
    let mut cli = BzOpts::new();
    // Print opening line
    {
        let descr = "bzip2, a block-sorting file compressor.";
        let created = "14-Jan-2023";
        println!("{}  Rust version {}, {}", descr, VERSION, created);
    }

    let args = std::env::args().skip(1);
    for mut arg in args {
        if arg.starts_with("--") {
            match arg.as_str() {
                "--help" => help(),
                "--decompress" => {
                    cli.op_mode = Mode::Unzip;
                }
                "--compress" => {
                    cli.op_mode = Mode::Zip;
                }
                "--keep" => cli.keep_input_files = true,
                "--force" => cli.force_overwrite = true,
                "--test" => cli.op_mode = Mode::Test,
                "--stdout" => cli.output = Output::Stdout,
                "--quiet" => cli.verbose = Verbosity::Quiet,
                "--verbose" => cli.verbose = Verbosity::Errors,
                "--license" => license(),
                "--version" => version(),
                "--small" => cli.small = true,
                "--fast" => cli.block_size = 1,
                "--best" => cli.block_size = 9,

                "--simple" => cli.algorithm = Algorithms::Simple,
                "--julian" => cli.algorithm = Algorithms::Julian,
                "--sais" => cli.algorithm = Algorithms::Sais,
                "--big" => cli.algorithm = Algorithms::Big,
                "--parallel" => cli.algorithm = Algorithms::Parallel,
                other => eprintln!("Bad command line argument: {}", other),
            }
        } else if arg.starts_with('-') {
            arg.remove(0);
            while !arg.is_empty() {
                if arg.starts_with("vvvvv") {
                    cli.verbose = Verbosity::Trace;
                    arg.remove(0);
                    arg.remove(0);
                    arg.remove(0);
                    arg.remove(0);
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with("vvvv") {
                    cli.verbose = Verbosity::Debug;
                    arg.remove(0);
                    arg.remove(0);
                    arg.remove(0);
                    arg.remove(0);
                    continue;
                } else if arg.starts_with("vvv") {
                    cli.verbose = Verbosity::Info;
                    arg.remove(0);
                    arg.remove(0);
                    arg.remove(0);
                    continue;
                } else if arg.starts_with("vv") {
                    cli.verbose = Verbosity::Warnings;
                    arg.remove(0);
                    arg.remove(0);
                    continue;
                } else if arg.starts_with('v') {
                    cli.verbose = Verbosity::Errors;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('h') {
                    help();
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('d') {
                    cli.op_mode = Mode::Unzip;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('z') {
                    cli.op_mode = Mode::Zip;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('k') {
                    cli.keep_input_files = true;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('f') {
                    cli.force_overwrite = true;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('t') {
                    cli.op_mode = Mode::Test;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('c') {
                    cli.output = Output::Stdout;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('q') {
                    cli.verbose = Verbosity::Quiet;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('L') {
                    license();
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('V') {
                    version();
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('s') {
                    cli.small = true;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('1') {
                    cli.block_size = 1;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('2') {
                    cli.block_size = 2;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('3') {
                    cli.block_size = 3;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('4') {
                    cli.block_size = 4;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('5') {
                    cli.block_size = 5;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('6') {
                    cli.block_size = 6;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('7') {
                    cli.block_size = 7;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('8') {
                    cli.block_size = 8;
                    arg.remove(0);
                    continue;
                }
                if arg.starts_with('9') {
                    cli.block_size = 9;
                    arg.remove(0);
                    continue;
                }
            }
        } else {
            cli.files.push(arg);
        };
    }
    // Set the log level
    match cli.verbose {
        Verbosity::Quiet => log::set_max_level(log::LevelFilter::Off),
        Verbosity::Errors => log::set_max_level(log::LevelFilter::Error),
        Verbosity::Warnings => log::set_max_level(log::LevelFilter::Warn),
        Verbosity::Info => log::set_max_level(log::LevelFilter::Info),
        Verbosity::Debug => log::set_max_level(log::LevelFilter::Debug),
        Verbosity::Trace => log::set_max_level(log::LevelFilter::Trace),
    };
    cli
}

/// Prints help information
fn help() {
    println!(
        "
   usage: bzip2 [flags and input files in any order]

   -h --help           print this message
   -d --decompress     force decompression
   -z --compress       force compression
   -k --keep           keep (don't delete) input files
   -f --force          overwrite existing output files
   -t --test           test compressed file integrity
   -c --stdout         output to standard out
   -q --quiet          suppress noncritical error messages
   -v --verbose        be verbose (a 2nd -v gives more)
   -L --license        display software version & license
   -V --version        display software version & license
   -s --small          use less memory (at most 2500k)
   -1 .. -9            set block size to 100k .. 900k
   --fast              alias for -1
   --best              alias for -9
   
    If invoked as `bzip2', default action is to compress.
              as `bunzip2',  default action is to decompress.
              as `bzcat', default action is to decompress to stdout.

   If no file names are given, bzip2 compresses or decompresses
   from standard input to standard output.  You can combine
   short flags, so `-v -4' means the same as -v4 or -4v, &c.

   Temporarily, you can specify one of these alogrithms for the BWT
     --simple
     --julian
     --sais
     --big
     --parallel
     -vvvvv (trace level debugging information)
   "
    );
    exit(0);
}

/// Official license statement for Bzip2
fn license() {
    println!(
        "
   bzip2, a block-sorting file compressor.
   Copyright (C) 1996-2010 by Julian Seward; 2010-2023 by various.
 
   This program is free software; you can redistribute it and/or modify
   it under the terms set out in the LICENSE file, which is included
   in the bzip2 source distribution.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   LICENSE file for more details."
    );
    exit(0);
}

fn version() {
    println!("Version: {}", VERSION);
    exit(0);
}

/// Define the alternate compression algorithms
#[derive(Clone, Debug, PartialEq, Eq, clap::Subcommand)]
pub enum Algorithms {
    /// Use original Bzip2 Burrow Wheeler Transform algorithm when compressing
    Julian,
    /// Use SAIS based Burrow Wheeler Transform algorithm when compressing
    Sais,
    /// Use simple Burrow Wheeler Transform algorithm when compressing
    Simple,
    // Parallel - uses custom BWT sorting alorithm with Rayon when compressing
    Parallel,
    // Big sequential - uses custom BWT sorting alorithm without Rayon
    Big,
}
/*
/// Defines a "fallback" mode for worst case data - may be depricated
#[derive(Debug)]
pub enum WorkFactor {
    Normal = 30,
    Fallback = 1,
}

/// Define all user settable options to control program behavior
#[derive(Debug)]
pub struct BzOptsOld {
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

impl BzOptsOld {
    /// Set default parameters on program start
    pub fn new() -> Self {
        Self {
            file: None,
            block_size: 9,
            op_mode: Mode::Zip,
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

    /// Sets verbosity. -v1 shows very little, -v5 is chatty
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
fn license_old() -> String {
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
pub fn init_bz_opts(bz_opts_old: &mut BzOptsOld) {
    let args = Args::parse();

    if args.filename.is_some() {
        bz_opts_old.file = Some(args.filename.as_ref().unwrap().to_string())
    };

    if args.compress {
        bz_opts_old.op_mode = Mode::Zip
    };

    if args.decompress {
        bz_opts_old.op_mode = Mode::Unzip
    };

    bz_opts_old.force_overwrite = args.force;

    bz_opts_old.keep_input_files = args.keep;

    if args.stdout {
        bz_opts_old.output = Output::Stdout
    };

    if args.workfactor {
        bz_opts_old.work_factor = WorkFactor::Fallback
    };

    if args.small {
        bz_opts_old.block_size = 2
    };

    // if args.test {
    //     bz_opts.op_mode = Mode::Test
    // };

    if args.fast {
        bz_opts_old.block_size = 2
    };

    if args.best {
        bz_opts_old.block_size = 9
    };

    // Set the log level
    match args.v {
        0 => log::set_max_level(log::LevelFilter::Off),
        1 => log::set_max_level(log::LevelFilter::Error),
        2 => log::set_max_level(log::LevelFilter::Warn),
        3 => log::set_max_level(log::LevelFilter::Info),
        4 => log::set_max_level(log::LevelFilter::Debug),
        _ => log::set_max_level(log::LevelFilter::Trace),
    };

    // NOTE: This overwrites the best and small flags!
    if args.block_size.is_some() {
        bz_opts_old.block_size = args.block_size.unwrap()
    };

    if args.license {
        info!("{}", license_old())
    };

    bz_opts_old.iterations = args.iterations;

    bz_opts_old.algorithm = args.algorithm.unwrap_or(Algorithms::Julian);

    // Below we report initialization status to the user
    info!("---- Bzip2 Initialization Start ----",);
    info!("Verbosity set to {}", log::max_level());
    //log::trace!("Testing trace level");
    info!("Operational mode set to {}", bz_opts_old.op_mode);
    match &bz_opts_old.file {
        Some(s) => info!("Getting input from the file {}", s),
        None => warn!("Sending output to stdout"),
    }
    info!("Block size set to {}", bz_opts_old.block_size);
    if bz_opts_old.force_overwrite {
        info!("Forcing file overwriting")
    };
    if bz_opts_old.keep_input_files {
        info!("Keeping input files")
    };
    if bz_opts_old.iterations != 4 {
        info!("Iterations set to {}", bz_opts_old.iterations)
    };
    info!("---- Bzip2 Initialization End ----\n");
}
 */
