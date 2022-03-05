/// bzip2 options - structs and impls and read & parse command line args
use std::{cmp::PartialOrd, fmt::Display, fmt::Formatter};

/// Used for logs and reporting
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Verbosity {
    Quiet,
    Errors,
    Normal,
    Chatty,
}
/// Verbosity: Used for logs and reporting
pub type V = Verbosity;
impl Display for V {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
/// Defines three operational modes
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
/// Defines two output channels
pub enum Output {
    File,
    Stdout,
}
impl Display for Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
/// Defines a "fallback" mode for worst case data - may be depricated
pub enum WorkFactor {
    Normal = 30,
    Fallback = 1,
}

#[derive(Debug)]
/// Defines all user settable options to control program behavior
pub struct BzOpts {
    /// Optional name of file to read for input
    pub file: Option<String>,
    /// Maximum input block size to process during each loop
    pub block_size: u8,
    /// User feedback level setting
    pub verbosity: Verbosity,
    /// Operation mode setting
    pub op_mode: Mode,
    /// Don't remove input files after processing
    pub keep_input_files: bool,
    /// Optional setting used for oddly constructed data - may be depricated
    pub work_factor: WorkFactor,
    /// Silently overwrite existing files with the same name
    pub force_overwrite: bool,
    /// Location where output is sent
    pub output: Output,
    /// Current status of progress
    pub status: Status,
}

impl BzOpts {
    /// Instanciated on program start - sets default parameters
    pub fn new() -> Self {
        Self {
            file: None,
            block_size: 9,
            verbosity: V::Normal,
            op_mode: Mode::Test,
            keep_input_files: false,
            work_factor: WorkFactor::Normal,
            force_overwrite: false,
            output: Output::File,
            status: Status::Init,
        }
    }
}

#[derive(Debug)]
pub enum Status {
    Init,
    Ok,
    NoData,
}
