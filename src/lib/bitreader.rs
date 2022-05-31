use std::{fs::File, io::Read};

#[derive(Debug)]
pub enum ReadError {
    Size,
}

/// Reads a Bzip2 file and allows reading a specified number of bits
#[derive(Debug)]
pub struct BitReader {
    buf: Vec<u8>,
    buf_size: usize,
    bytes_left: usize,
    byte_index: usize,
    bit_index: usize,
    file: File,
}

impl BitReader {
    /// Called to create a new bitReader
    pub fn new(file: File, file_length: usize, buf_size: usize) -> Self {
        Self {
            buf: vec![0; buf_size],
            buf_size,
            bytes_left: 0,
            byte_index: 0,
            bit_index: 0,
            file,
        }
    }

    /// Internal bitstream read function that tries to keep the read buffer in good shape
    /// NOTE: Reading is not done in buffered chunks properly
    fn check_stream(&mut self) -> bool {
        if self.bytes_left == 0 && self.bit_index == 0 {
            self.bytes_left = self
                .file
                .read(&mut self.buf)
                .expect("Oops, can't read more bytes"); // needs better error msg!
            if self.bytes_left == 0 {
                return false;
            }
        }
        true
    }

    /// Debugging function. Report current position.
    pub fn loc(&mut self) -> String {
        format!("[{}.{}]", self.byte_index, self.bit_index)
    }

    /// Read 8 or less bits and return it in a u8 with leading zeros, or None if there is no data
    pub fn read8(&mut self, mut length: usize) -> Option<u8> {
        // Return no more than 8 bits
        length = length.min(8);

        // Aways check to see if we have data
        if !self.check_stream() {
            return None;
        }

        // Start by grabbing bits from the current queue byte position
        let mut byte = self.buf[self.byte_index % self.buf_size];

        // Left shift it to get rid of bits we may have already have used
        byte <<= self.bit_index;

        // Adjust the bit index by the number of bits we were able to read
        let bits_read = match self.bit_index {
            0 => 8.min(length),
            _ => (8 - self.bit_index).min(length),
        };
        self.bit_index += bits_read;

        // Adjust the byte index (and reset the bit index) if we used up the current byte
        if self.bit_index == 8 {
            self.byte_index += 1;
            self.bytes_left -= 1;
            self.bit_index = 0;
        }

        // Did we get enough bits? If so, return the data (right shifted).
        if length - bits_read == 0 {
            Some(byte >> ((8 - length) % 8))
        } else {
            // We need more bits. Get a new byte, shifted right by the number of bits
            // we already got so we don't clobber that info.  Then OR the new bits
            // onto the bits we already have.
            byte |= (self.buf[self.byte_index % self.buf_size] >> bits_read);

            // Then right shift to get rid of any bits we don't need
            if length < 8 {
                byte >>= 8 - (length);
            }

            // Update how many bits we have used (this will always be less than 8)
            self.bit_index = length - bits_read;

            // Return the new byte
            Some(byte)
        }
    }

    /// Read more than 8 bits and return it in a Vec<u8> with trailing padding (0s), not leading
    pub fn read8plus(&mut self, length: usize) -> Result<Vec<u8>, ReadError> {
        let mut out: Vec<u8> = vec![0; length as usize / 8];
        //for i in 0..(length as usize / 8) {
        for item in out.iter_mut().take(length as usize / 8) {
                if let Some(byte) = self.read8(8) {
                *item = byte
            };
        }
        if length % 8 > 0 {
            if let Some(byte) = self.read8(length % 8) {
                out.push(byte << (8 - (length % 8)))
            };
        }
        Ok(out)
    }
}
