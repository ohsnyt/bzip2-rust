//! The bwt_algorithms module forms the critical sorting subsystem for the Rust version of the standard BZIP2 library.
//!
//! BZIP2 uses the Burrow-Wheeler Transform (BWT) to prepare data for compression. This transform alters the data in such
//! a way that runs of similar bytes are more likely to occur. This allows for more effective compression.
//! 
//! The Burrow-Wheeler Transform requires "computationally expensive" sorting. Since different sorting algorithms are better
//! suited for different kinds of data, this module contains multiple sorting algorithms. (Currently two are employed.)
//! 
pub mod bwt_sort;
pub mod sais_fallback;
