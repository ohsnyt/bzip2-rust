use std::{fs::File, io::Read};

const BUFFER_SIZE: usize = 1024 * 1024;
const BIT_MASK: u8 = 0xff;

/// Reads a Bzip2 file and allows reading a specified number of bits
#[derive(Debug)]
pub struct BitReader<R> {
    buffer: Vec<u8>,
    bytes_read: usize,
    byte_index: usize,
    bit_index: usize,
    source: R,
}

impl<R: std::io::Read> BitReader<R> {
    /// Called to create a new bitReader
    pub fn new(source: R) -> Self {
        Self {
            buffer: vec![0; BUFFER_SIZE],
            bytes_read: 0,
            byte_index: BUFFER_SIZE,
            bit_index: 0,
            source,
        }
    }

    /// Check (and refill) buffer - true if we have data, false if there is no more
    fn have_data(&mut self) -> bool {
        // first time...
        // if self.bytes_read == 0 {
        //     let size = self
        //         .source
        //         .read(&mut self.buffer)
        //         .expect("Unble to read source data");
        //     if size == 0 {
        //         return false;
        //     } else {
        //         self.buffer.truncate(size);
        //         self.bytes_read += size;
        //         self.byte_index = 0;
        //         self.bit_index = 0;
        //     }
        // } else {
        if self.bytes_read == 0 || self.byte_index == self.buffer.len() {
            //if self.is_empty() {
            let size = self
                .source
                .read(&mut self.buffer)
                .expect("Unble to read source data");
            if size == 0 {
                return false;
            } else {
                self.buffer.truncate(size);
                self.bytes_read += size;
                self.byte_index = 0;
                self.bit_index = 0;
            }
        }
        //}
        true
    }

    /// Function to indicate buffer is empty (not necessarily the source)
    fn is_empty(&self) -> bool {
        (self.byte_index > self.buffer.len() - 1)
            || (self.byte_index == self.buffer.len() && self.bit_index == 0)
    }

    /// Return one bit
    pub fn bit(&mut self) -> Option<usize> {
        // If bit_index is == 0, check if we have a byte to read. Return None if we have no data
        if self.bit_index == 0 && !self.have_data() {
            return None;
        }
        let bit =
            (self.buffer[self.byte_index] & BIT_MASK >> self.bit_index) >> (7 - self.bit_index);
        self.bit_index += 1;
        self.bit_index %= 8;
        if self.bit_index == 0 {
            self.byte_index += 1;
        }
        Some(bit as usize)
    }

    /// Return Option<Bool> *true* if the next bit is 1, *false* if 0
    pub fn bool_bit(&mut self) -> Option<bool> {
        self.bit().map(|bit| bit == 1)
    }

    /// Return Option<usize> of the next n bits
    pub fn bint(&mut self, mut n: usize) -> Option<usize> {
        /*
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
            result = ((self.buffer[self.byte_index] & BIT_MASK >> self.bit_index)
                >> (8 - self.bit_index - needed)) as usize;
            self.bit_index += needed;
            if self.bit_index / 8 > 0 {
                self.byte_index += 1;
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
            result = result << 8 | (self.buffer[self.byte_index]) as usize;
            self.byte_index += 1;
            n -= 8;
        }
        // If we still need a partial byte, get whatever bits we still need.
        if n > 0 {
            // Checking always for data
            if !self.have_data() {
                return None;
            }
            // Get the remaining bits
            result = result << n | (self.buffer[self.byte_index] >> (8 - n)) as usize;
            // Adjust indecies
            self.bit_index += n;
            if self.bit_index / 8 > 1 {
                self.byte_index += 1;
            }
            self.bit_index %= 8;
        }
        Some(result)
    }

    /// Read and return a bytes as an Option<u8>
    pub fn byte(&mut self) -> Option<u8> {
        self.bint(8).map(|byte| byte as u8)
    }

    /// Read and return a vec of n bytes as an Option<Vec<u8>>
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

    /// Debugging function. Report current position.
    pub fn loc(&self) -> String {
        format!("[{}.{}]", self.byte_index, self.bit_index)
    }
}

// Iterator is not currently used, but was tried with alternative factorings that proved slower.
impl<R> Iterator for BitReader<R>
where
    R: Read,
{
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        self.bit()
    }
}

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
    fn iter_test() {
        let x = [0b1000_0001_u8, 0b0100_1000].as_slice();
        let mut br = BitReader::new(x);
        assert_eq!(br.next(), Some(1));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(1));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(1));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(1));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), Some(0));
        assert_eq!(br.next(), None);
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
}
