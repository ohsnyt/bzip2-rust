use std::{
    fs::{File, Metadata},
    io::Read,
};
use super::options::{Verbosity::Errors, BzOpts};
use super::options::Verbosity::Normal;
use super::report::report;

#[derive(Debug)]
/// Struct used for reading data. Necessary for block processing.
pub struct Data<'a> {
    f_in: File,
    meta: Metadata,
    size: usize,
    buf: Vec<u8>,
    data_left: usize,
    bz_opts: &'a BzOpts,
}
/// Instantiate new data input instance, setting up output buffer.
impl<'a> Data<'a> {
    pub fn new(f_in: File, meta: Metadata, size: usize, data_left: usize, bz_opts: &'a BzOpts) -> Self {
        Self {
            f_in,
            meta,
            size,
            buf: vec![0; size],
            data_left,
            bz_opts,
        }
    }
}

impl Data<'_> {
    /// Read exactly one block of data (or less, if eof) for processing
    pub fn read(&mut self) -> Option<&Vec<u8>> {
        if self.data_left == 0 {return None}
        if self.data_left < self.size {
            self.size = self.data_left;
            self.buf.clear();
            self.buf = vec![0; self.size]
        };
        self.data_left -= self.size;
        match self.f_in.read_exact(&mut self.buf) {
            Ok(_) => return Some(self.buf.as_ref()),
            Err(_) => {
                report(self.bz_opts, Errors, "Error reading input file.");
                return None;
            }
        }
    }
}

/// Initialize file reading - get result as a a Data object, which supports reading
/// (by iteration) data by block size defined in bz_opts. Standard IO errors are reported
/// and returned.
pub fn init(bz_opts: &BzOpts) -> Result<Data<'_>, std::io::Error> {
    //first, get the file name from the options
    let mut f = String::new();
    if bz_opts.file.is_none() {
        report(&bz_opts, Normal, "Using >test.txt< as the input file.");
        f = "test.txt".to_string()
    } else {
        f = bz_opts.file.as_ref().unwrap().to_string()
    }

    let f_in = match File::open(&f) {
        Ok(file) => file,
        Err(e) => {
            report(
                &bz_opts,
                Errors,
                &format!("Cannot read from the file {}", f),
            );
            return Err(e);
        }
    };

    let meta = match f_in.metadata() {
        Ok(m) => m,
        Err(e) => {
            report(
                &bz_opts,
                Errors,
                &format!("Cannot obtain metadata on {}", f),
            );
            return Err(e);
        }
    };
    let size: usize = meta
        .len()
        .min((bz_opts.block_size as u32 * 100_000).into())
        .try_into()
        .unwrap();
        let data_left = meta.len().try_into().unwrap();
    Ok(Data::new(f_in, meta, size, data_left, bz_opts))
}
