use std::process::exit;
use std::{fmt::Display, fmt::Formatter};

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

                other => eprintln!("Unexpected command line argument: {}", other),
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
                    // And remove any excess v's
                    while arg.starts_with('v') {
                        arg.remove(0);
                    }
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
                eprintln!("Unexpected command line argument: {}", arg);
                help()
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
    println!("Version: {}, written in Rust", VERSION);
    exit(0);
}

/*
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
*/
