//! The huffman module generates the bistream for the Rust version of the standard BZIP2 library. Decoding the 
//! huffman data happens in the decompress function.
//!
//! BZIP2 is a block-oriented approach to compress data. 
//! 
//! Huffman encoding is used in lieu of arithmetic encoding because of an historical problem with licensing restrictions. 
//! While that has been resolved in more recent years, the BZIP2 standard was set based on the huffman standard.
//! 
//! The huffman coding algorithm as used by BZIP2 is both block and chunk oriented. The data stream is broken into blocks of
//! approximately 100-900k (at the RLE1 stage), based on parameters specified by the user. Within each block, chunks of 50 
//! bytes of data are encoded separately using one of six huffman tables. This allows for higher compression ratios compared to
//! using one huffman table per block (or for the entire file).
//! 
//! The process of encoding and decoding each block is inherently sequential and does not benefit from multithreading.
//! 
//! 

pub mod huffman;
pub mod huffman_code_from_weights;
