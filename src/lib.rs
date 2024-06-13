//! Rust version of the standard BZIP2 library.
//!
//! Version 0.5.0
//! 
//! This Rust version of BZIP2 is based on the standard BZIP2 C implementation. This does NOT implement the library calls. Why? The C library
//! calls require that the caller allocate memory and pass that to the library. The would require signficant unsafe code to manage the memory,
//! and I am not prepared to implement that yet.
//!
//! Given that caveat, this program does
//! - Provide fast, safe compression and decompression of files using the bzip2 format.
//! - Utilize multi-core multi-threaded processing. 
//! - Contain SA-IS sorting to improve compression speeds on repetative data such as genome sequences.
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
//! NOTES: 
//! - The C version is very well written in terms of optimization. Julian Seward clearly worked hard on that code. But documentation... well this is much more
//! documented than the C version.
//! - Developer feedback is welcome. If you have suggestions for improvement, please let me know!
//! - This version compresses about 25% slower than the C version for tiny files. This is faster on larger files.
//! - It is particularly faster when using the SA-IS sorting algorithm as the fallback sorting algorithm.
//! - This version is about 25% slower than the C version for decompression. That said, BZIP2 decompression is pretty fast.
//!
pub mod bitstream;
pub mod compression;
pub mod huffman_coding;
pub mod bwt_algorithms;
pub mod tools;
