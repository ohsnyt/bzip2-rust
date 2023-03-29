//! The compress module manages the compression side of the the Rust version of the standard BZIP2 library.
//!
//! BZIP2 compression happens in the following steps:
//! - Run Length Encoding 1: Compress all runs of 4-255 identical bytes.
//! - Burrow Wheeler Transform: Sort the data to increase the probability of runs of identical bytes.
//! - Move To Front transform: Increase the frequency of lower byte values, and thereby decrease the frequency of other byte values.
//! - Run Length Encoding 2: Compress all runs of the zero byte.
//! - Huffman coding: Encode frequent byte values using smaller bit codes and less frequent byte values with longer bit codes.
//! 
//! While the initial RLE1 compression is probably not necessary, it is a legacy of the original implemention and must be preserved.
//! 
//! The BZIP2 huffman stage actually makes four passes over the data to improve the compression ratio. Additionally, six different
//! huffman tables are generated for each block of data, and every chunk of 50 bytes within that block is analyzed to determine which 
//! huffman table will result in the best compression ratio for that chunk.
//! 
//! Decompression is single threaded. It follows the inverse of the compression process.
//! - Huffman decoding.
//! - RLE 2: Expand all runs of the zero byte.
//! - MTF transform: Convert from the Move-To-Front indecies to the symbols represented by the indecies.
//! - BWT reversal: Restore the original data from the BWT transform.
//! - RLE 1: Expand all runs of 4+ identical bytes.
//! 

pub mod compress;
pub mod compress_block;
pub mod decompress;