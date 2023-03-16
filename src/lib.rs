//! Rust version of the standard BZIP2 library.
//!
//! Version 0.3.0
//! (This version does NOT contain API calls.)
//!
//! Provides fast, safe compression and decompression of files using the bzip2 format.
//!
//! Utilizes multi-core parallelism. Contains enhancements from the original c version
//! that allows for much faster processing of highly repetative data such as genetic
//! sequences.
//!
//! Basic usage to compress a files is as follows:
//!
//! `$> bzip2 -z test.txt`
//!
//! This will compress the file and create the file test.txt.bz2.
//! The original file will be deleted.
//!
pub mod bitstream;
pub mod compression;
pub mod huffman_coding;
pub mod bwt_algorithms;
pub mod tools;
