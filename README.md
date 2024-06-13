# bzip2-rust
Rust implementation of Bzip2 library. This is beta stage. Use at your own risk. This may be used under either the Apache or MIT license.

*Block specific decoding modes are not yet implemented.*

The goal of the executable is to allow for near 100% compatibility with the existing C version of the Bzip2 executable. However the internal algorithms differ. 

The largest internal differences are that this version implements 
1) an alternative sorting algorithm optimized for highly repetitive data such as gene sequencing information. You should find signficant speed enhancements for that type of data.
2) multi-threading to optimize speed on multi-core processors

The C version of Bzip2 was very memory efficient - a key factor for that era. This version tries to be careful with memory usage, but does not prioritize memory usage over speed.

The C version of Bzip2 contains a library and several stand-alone tools to work with files (including damaged, compressed files). I do *not* attempt to reproduce those tools.

David Snyder, June 2024.
