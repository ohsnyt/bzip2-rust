//! The tools module provides several helper functions for the Rust version of the standard BZIP2 library. 
//!
//! BZIP2 is a block-oriented approach to compress data. 
//! 
//! The tools are:
//! - cli: Command line interface for BZIP2.
//! - crc: CRC32 checksum for BZIP2, both block and stream versions.
//! - freq_count: Frequency count for BZIP2.
//! - rle1: Run-Length-Encoding phase 1 for BZIP2.
//! - rle2_mtf: Move-To-Front transform and Run-Length-Encoding phase 2 (integrated for speed) for BZIP2.
//! - symbol_map: Decode the symbol map used in BZIP2.
//! 
pub mod cli;
pub mod crc;
pub mod freq_count;
pub mod rle1;
pub mod rle2_mtf;
pub mod symbol_map;

