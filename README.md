# bzip2-rust
Rust implementation of Bzip2 library. This is later alpha stage. Use at your own risk. *Some features are not yet implemented.*

This library also fseeks to contribute to a rust implementation that allows integration through a C language interface. Thus the API will be C FFI. The internal code will be Rust.

The goal of the library is to allow for 100% compatibility with the existing C version of Bzip2 - thus compiling with this library vs compiling with the existing C library should yeild the same output for the user.

David Snyder, March 2023.
