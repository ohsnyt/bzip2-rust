//! The huffman module compresses the MTF/RLE2 data into a bistream. Decoding huffman (decompression) data happens in the decompress function.
//!
//! Huffman encoding is used in lieu of arithmetic encoding because of an historical problem with licensing restrictions. 
//! While that has been resolved in more recent years, the BZIP2 standard was set based on the huffman standard.
//! 
//! The huffman coding algorithm as used by BZIP2 is both block and chunk oriented. The data stream is broken into blocks of
//! approximately 100-900k (at the RLE1 stage), based on parameters specified by the user. Within each block, chunks of 50 
//! bytes of data are encoded separately using one of six huffman tables. This allows for higher compression ratios compared to
//! using one huffman table per block (or for the entire file).
//! 
//! The process of huffman encoding a block is inherently sequential and does not benefit from multithreading.
//! 
//! 

pub mod huffman;
pub mod huffman_code_from_weights;
