//! The bitstream module forms the I/O subsystem for the Rust version of the standard BZIP2 library.
//!
//! BZIP2 is a block-oriented approach to compress data.
//! 
//! The original version of BZIP2, being single-threaded, was able to write the bitstream from start to finish.
//! This multi-threaded version required that each block pass the huffman encoded data to the final aggregator, which
//! would then write the continuous output stream.
//! 
//! This I/O subsystem is designed to efficiently interface with the other modules within BZIP2. It is not intended for
//! more general use. (It has not been generalized to handle a wider variety of calles that might be necessary in other applications.)
//! 
pub mod bitwriter;
pub mod bitpacker;
pub mod bitreader;
