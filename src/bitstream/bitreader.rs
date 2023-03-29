//! BitReader: A module for the Rust version of the standard BZIP2 library.
//!
//! Reads a packed bitstream for the block-oriented deconstruction of BZIP2 compressed files.
//! 
//! NOTE: This module can read from any I/O source that supports the read() call.
//!

const BUFFER_SIZE: usize = 1024 * 1024;
const BIT_MASK: u8 = 0xff;

/// Reads a binary Bzip2 file.
#[derive(Debug)]
pub struct BitReader<R> {
    buffer: Vec<u8>,
    cursor: usize,
    bit_index: usize,
    source: R,
}

impl<R: std::io::Read> BitReader<R> {
    /// Creates a new bitReader (with a 1Mbyte buffer).
    pub fn new(source: R) -> Self {
        Self {
            buffer: vec![0; BUFFER_SIZE],
            cursor: BUFFER_SIZE,
            bit_index: 0,
            source,
        }
    }

    /// Check (and refill) buffer. Returns true if we have data, false if there is no more
    fn have_data(&mut self) -> bool {
        // Only try to read more data when the buffer length is equal to the buffer cursor location
        if self.cursor == self.buffer.len() {
            let size = self
                .source
                .read(&mut self.buffer)
                .expect("Unable to read source data");
            // If nothing came back from our read attempt, then we have no more data.
            if size == 0 {
                return false;
            } else {
                // Adjust the buffer if we read less than the buffer size
                self.buffer.truncate(size);
                // Reset the cursor and bit index
                self.cursor = 0;
                self.bit_index = 0;
            }
        }
        true
    }

    /// Return bit as Option<usize> (1 or 0), or None if there is no more data to read
    pub fn bit(&mut self) -> Option<usize> {
        // If bit_index is == 0, check if we have a byte to read. Return None if we have no data
        if self.bit_index == 0 && !self.have_data() {
            return None;
        }
        // Otherwise return the bit as an Some(usize)
        let bit =
            (self.buffer[self.cursor] & BIT_MASK >> self.bit_index) >> (7 - self.bit_index);
        self.bit_index += 1;
        self.bit_index %= 8;
        if self.bit_index == 0 {
            self.cursor += 1;
        }
        Some(bit as usize)
    }

    /// Return Option<Bool> *true* if the next bit is 1, *false* if 0, consuming the bit, 
    /// or None if there is no more data to read
    pub fn bool_bit(&mut self) -> Option<bool> {
        self.bit().map(|bit| bit == 1)
    }

    /// Return Option<usize> of the next n bits, or None if there is no more data to read. 
    pub fn bint(&mut self, mut n: usize) -> Option<usize> {
        /*
        This is used primarilyl to return signatures and crc values. For example, if a crc
        value is stored on the stream as a u32, then bint(32) will return the crc value in
        Some(usize).

        This is optimized to read as many bits as possible for each read.
        First, look to see if we have less than 8 bits in the current byte. If so, get
        those. Then get full bytes as needed to fulfill the request. Lastly, get a
        partial byte to complete the request.
        */
        // Prepare the usize for returning
        let mut result = 0_usize;

        // Test if we have a partial byte of data. If we do, read from it.
        if self.bit_index > 0 {
            // Set up to read the minimum of the partial byte and what we need to read
            let needed = n.min(8 - self.bit_index);

            // Get what we need/can from this partial byte
            result = ((self.buffer[self.cursor] & BIT_MASK >> self.bit_index)
                >> (8 - self.bit_index - needed)) as usize;
            self.bit_index += needed;
            if self.bit_index / 8 > 0 {
                self.cursor += 1;
            }
            self.bit_index %= 8;

            // See if we got all we needed.
            if n == needed {
                // Return if so.
                return Some(result);
            } else {
                // Else adjust what we still need and try to read more data.
                n -= needed;
            }
        }
        // If we are here, we need more data. Get as many full bytes as we need.
        while n >= 8 {
            // Checking always for data
            if !self.have_data() {
                return None;
            }
            result = result << 8 | (self.buffer[self.cursor]) as usize;
            self.cursor += 1;
            n -= 8;
        }
        // If we still need a partial byte, get whatever bits we still need.
        if n > 0 {
            // Checking always for data
            if !self.have_data() {
                return None;
            }
            // Get the remaining bits
            result = result << n | (self.buffer[self.cursor] >> (8 - n)) as usize;
            // Adjust indecies
            self.bit_index += n;
            if self.bit_index / 8 > 1 {
                self.cursor += 1;
            }
            self.bit_index %= 8;
        }
        Some(result)
    }

    /// Returns a byte as an Option<u8>, or None if there is no more data to read. This is
    /// a convenience function, and calls bint(8).
    pub fn byte(&mut self) -> Option<u8> {
        self.bint(8).map(|byte| byte as u8)
    }

    /// Returns an Option<Vec<u8>> of n bytes, or None if there is no more data to read. This
    /// is a convenience function, and calls byte n times.
    pub fn bytes(&mut self, mut n: usize) -> Option<Vec<u8>> {
        let mut result: Vec<u8> = Vec::with_capacity(n);

        while n > 0 {
            if let Some(byte) = self.byte() {
                result.push(byte);
                n -= 1;
            }
        }
        Some(result)
    }

    /// Debugging function. Report current position in the buffer.
    pub fn loc(&self) -> String {
        format!("[{}.{}]", self.cursor, self.bit_index)
    }
}

/*
Note: I tried several refactorings to use an iterator to read bits for the above functions,
but this code above proved faster than any iterator I could devise.
 */

#[cfg(test)]
mod test {
    use super::BitReader;

    #[test]
    fn basic_test() {
        let x = [0b10000001_u8].as_slice();
        let mut br = BitReader::new(x);
        assert_eq!(br.bit(), Some(1));
        assert_eq!(br.bit(), Some(0));
        assert_eq!(br.bit(), Some(0));
        assert_eq!(br.bit(), Some(0));
        assert_eq!(br.bit(), Some(0));
        assert_eq!(br.bit(), Some(0));
        assert_eq!(br.bit(), Some(0));
        assert_eq!(br.bit(), Some(1));
        assert_eq!(br.bit(), None);
    }

    #[test]
    fn bint_test() {
        let x = [0b00011011].as_slice();
        let mut br = BitReader::new(x);
        assert_eq!(br.bint(5), Some(3));
        assert_eq!(br.bint(1), Some(0));
        assert_eq!(br.bint(2), Some(3));
    }

    #[test]
    fn byte_test() {
        let x = "Hello, world!".as_bytes();
        let mut br = BitReader::new(x);
        assert_eq!(br.byte(), Some('H' as u8));
        assert_eq!(br.byte(), Some('e' as u8));
        assert_eq!(br.byte(), Some('l' as u8));
        assert_eq!(br.byte(), Some('l' as u8));
    }

    #[test]
    fn bytes_test() {
        let x = "Hello, world!".as_bytes();
        let mut br = BitReader::new(x);
        assert_eq!(br.bytes(5), Some("Hello".as_bytes().to_vec()));
        }

    #[test]
    fn loc_test() {
        let x = "Hello, world!".as_bytes();
        let mut br = BitReader::new(x);
        br.bytes(5);
        br.bit();
        assert_eq!(br.loc(), "[5.1]");
        }
    
    #[test]
    fn bool_bit_test() {
        let x = [0b01010000].as_slice();
        let mut br = BitReader::new(x);
        assert_eq!(br.bool_bit(), Some(false));
        assert_eq!(br.bool_bit(), Some(true));
        assert_eq!(br.bool_bit(), Some(false));
        assert_eq!(br.bool_bit(), Some(true));
        assert_eq!(br.bool_bit(), Some(false));
        assert_eq!(br.bool_bit(), Some(false));
        assert_eq!(br.bool_bit(), Some(false));
        assert_eq!(br.bool_bit(), Some(false));
    }
}
