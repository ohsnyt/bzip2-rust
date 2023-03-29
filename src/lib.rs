//! Rust version of the standard BZIP2 library.
//!
//! Version 0.4.0
//! (This version does NOT contain API calls.)
//!
//! - Provides fast, safe compression and decompression of files using the bzip2 format.
//! - Utilizes multi-core multi-threaded processing. 
//! - Contains SA-IS sorting to improve compression speeds on repetative data.
//!
//! Basic usage to compress a files is as follows:
//!
//! ```
//! `$> bzip2 -z test.txt`
//! ```
//! This will compress the file and create the file test.txt.bz2.
//! 
//! 
//! Basic usage to decompress a files is as follows:
//!
//! ```
//! `$> bzip2 -d test.txt.bz2`
//! ```
//! 
//! Help is available by entering:
//!
//! ```
//! `$> bzip2 --help`
//! ```
//!
//!
//#![doc(html_logo_url = "https://github.com/ohsnyt/bzip2-rust/blob/Use-sais3-as-fallback/Oh%20Snyt%20Famous%20Code%20logo.png?raw=true")]
#![doc(html_logo_url = "file:///Users/david/Downloads/Oh%20Snyt%20Famous%20Code%20logo.png?")]
pub mod bitstream;
pub mod compression;
pub mod huffman_coding;
pub mod bwt_algorithms;
pub mod tools;
