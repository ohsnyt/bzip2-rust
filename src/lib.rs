//! Rust version of the standard BZIP2 library.
//!
//! Version 0.5.0
//!
//! - Provides fast, safe compression and decompression of files using the bzip2 format.
//! - Utilizes multi-core multi-threaded processing. 
//! - Contains SA-IS sorting to improve compression speeds on repetative data.
//!
//! Basic usage to compress a files is as follows:
//! 
//! `bzip2 -z test.txt`
//! 
//! This will compress the file and create the file test.txt.bz2.
//! 
//! 
//! Basic usage to decompress a files is as follows:
//! 
//!  `bzip2 -d test.txt.bz2`
//! 
//! 
//! Help is available by entering:
//! 
//! `bzip2 --help`
//! 
//! NOTES: This version compresses slower than the C version for smaller files. It is faster on larger files.
//! It is particularly faster when using the SA-IS sorting algorithm as the fallback sorting algorithm.
//! 
//! This version is also slower on decompression. I have not spent much time optimizing that aspect. 
//!
pub mod bitstream;
pub mod compression;
pub mod huffman_coding;
pub mod bwt_algorithms;
pub mod tools;
