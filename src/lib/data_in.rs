use super::options::Verbosity::Normal;
use super::options::{BzOpts, Verbosity::Errors};
use super::report::report;
use std::{
    fs::{File, Metadata},
};

#[derive(Debug)]
/// Struct used for reading data. Necessary for block processing.
pub struct Data {
    pub f_in: File,
    pub meta: Metadata,
    pub size: usize,
    pub data_left: usize,
    //pub is_last: bool,
}
/// Instantiate new data input instance, setting up output buffer.
impl Data {
    pub fn new(
        f_in: File,
        meta: Metadata,
        size: usize,
        data_left: usize,
    ) -> Self {
        Self {
            f_in,
            meta,
            size,
            data_left,
            //is_last: false,
        }
    }
}

/// Initialize file reading - get result as a a Data object, which supports reading
/// (by iteration) data by block size defined in bz_opts. Standard IO errors are reported
/// and returned.
pub fn init(bz_opts: &BzOpts) -> Result<Data, std::io::Error> {
    //first, get the file name from the options
    let mut f = "test.txt".to_string();
    if bz_opts.file.is_none() {
        report(bz_opts, Normal, "Using >test.txt< as the input file.");
    } else {
        f = bz_opts.file.as_ref().unwrap().to_string()
    }

    let f_in = match File::open(&f) {
        Ok(file) => file,
        Err(e) => {
            report(
                bz_opts,
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
                bz_opts,
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
    let data_left:usize = meta.len() as usize - size;
    
    Ok(Data::new(f_in, meta, size, data_left))
}
