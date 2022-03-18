use std::{fs::File, io::Read};

#[derive(Debug)]
pub enum ReadError {
    Size,
}

/// Reads a Bzip2 file and allows reading a specified number of bits
#[derive(Debug)]
pub struct BitReader {
    queue: Vec<u8>,
    used: usize,
    file: File,
    position: usize,
}

impl BitReader {
    /// Called to create a new bitReader
    pub fn new(file: File) -> Self {
        Self {
            queue: Vec::with_capacity(100000),
            used: 0,
            file,
            position: 0,
        }
    }

    /// Internal bitstream read function that tries to keep the read buffer in good shape
    /// NOTE: Reading is not done in buffered chunks properly
    fn clean_stream(&mut self) {
        if self.used >= 8 {
            let bytes = self.used / 8;
            self.position += bytes * 8;
            self.used %= 8;
            self.queue.drain(..bytes);
        }
        if self.queue.len() < 500 {
            let mut buf = Vec::new();
            //buf.reserve(50000); // I need to find a better way to read chunks
            self.file.read_to_end(&mut buf).expect("oops"); // needs better error msg!
            self.queue.append(&mut buf);
        }
    }

    /// Debugging function. Report current position.
    pub fn loc(&mut self) -> String {
        format!("[{}.{}]", (self.position + self.used) / 8, (self.position  + self.used)% 8,)
    }

    /// Read 8 or less bits and return it in a u8 with leading zeros
    /// Error if size > 8
    pub fn read8(&mut self, length: u32) -> Result<u8, ReadError> {
        if length > 8 {
            return Err(ReadError::Size);
        }
        // Aways start with a clean slate
        self.clean_stream();
        // Get the beginning of the queue and remove the "used" bits
        let mut out = self.queue[0] << (self.used % 8);
        // See if we need more bits
        if length > 8 - self.used as u32 {
            // Get a new byte, shift it right so we don't clobber the good bits
            //  and OR this new shifted byte onto the good bits we have
            out |= self.queue[1] >> (8 - self.used);
        }
        // Update how many bits we have used
        self.used += length as usize;
        // shift any excess bits
        out >>= (8 - length) % 8;
        Ok(out)
    }

    /// Read more than 8 bits and return it in a u8 with trailing padding (0s)
    /// Not yet checking or EOF problems
    pub fn read8plus(&mut self, length: u32) -> Result<Vec<u8>, ReadError> {
        let mut out: Vec<u8> = Vec::new();
        for _ in 0..(length / 8) {
            match self.read8(8) {
                Ok(byte) => out.push(byte),
                Err(e) => return Err(e),
            };
        }
        if length % 8 > 0 {
            match self.read8(length % 8) {
                Ok(byte) => out.push(byte),
                Err(e) => return Err(e),
            };
        }
        Ok(out)
    }
}
