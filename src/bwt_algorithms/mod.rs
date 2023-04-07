//! The bwt_algorithms form the critical sorting subsystem for the Rust version of the standard BZIP2 library.
//!
//! BZIP2 uses the Burrow-Wheeler Transform (BWT) to prepare data for compression. This transform alters the data in such
//! a way that runs of similar bytes are more likely to occur. This allows for more effective compression.
//! 
//! The Burrow-Wheeler Transform requires "computationally expensive" sorting. Since different sorting algorithms are better
//! suited for different kinds of data, this module contains multiple sorting algorithms. (Currently two are employed, but numerous
//! alternatives were tested.)
//! 
//! Of all the phases involved in BZIP2, this phase has the greatest impact on compression speed.
//! 
pub mod bwt_sort;
pub mod sais_fallback;
