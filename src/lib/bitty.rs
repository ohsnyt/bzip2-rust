// bitty.rs is a bit reader (and maybe writer) with iteration using a BufReader
// a set bit returns true, a clear bit returns false

use std::io::{BufReader, Read};

use log::error;

const MASK: u8 = 0b10000000;
// For some reason, queue sizes that are multiples of the read buffer size are problematic.
// There may be rare bug waiting in this number and the queue resizing
const QUEUE_SIZE: usize = 1048;

/// BitReader accepts input from files or slices &[u8] and returns bits, bytes and u32 values as requested.
#[derive(Debug)]
pub struct BitReader<R> {
    reader: BufReader<R>, 
    queue: Vec<u8>,
    byte_index: usize,
    bit_index: usize,
    is_empty: bool,
}

impl<R> BitReader<R>
where
    R: Read,
{
    /// Creates a new BitReader from a file handle (File::open(inputfile)?) or a slice &[[u8]]
    pub fn new(data: R) -> Self {
        Self {
            reader: BufReader::new(data),
            queue: vec![0; QUEUE_SIZE],
            byte_index: 0,
            bit_index: 0,
            is_empty: true,
        }
    }

    /// Private helper function to make sure the queue is valid and we have data to return n bits
    #[inline(always)]
    fn check_queue(&mut self, n: usize) {
        // If the queue low, try to get more from the bufreader
        if self.is_empty || self.byte_index + n / 8 >= self.queue.len() {
            // First reset the byte_index
            if self.is_empty {
                self.byte_index = 0;
            } else {
                // Shift the good bytes to the beginning of the queue
                for i in 0..self.queue.len() - self.byte_index {
                    self.queue.swap(i, i + self.byte_index)
                }
                // Temporarily leave the byte index above zero so we can get the right amount of data
                self.byte_index = self.queue.len() - self.byte_index;
            }

            // Try to get more data from the read buffer
            // A previous buffer read from an used buffer can result in a small queue. Reset the queue length.
            if self.queue.len() < QUEUE_SIZE {
                self.queue.extend_from_slice(&vec![0_u8; QUEUE_SIZE-self.queue.len()])
            }
            match self.reader.read(&mut self.queue[self.byte_index..]) {
                Ok(got) => {
                    // Got a valid response. First set the buffer length
                    self.queue.truncate(got + self.byte_index);
                    if got + self.byte_index > 0 {
                        // There was some data, set is_empty as false and reset the byte_index
                        self.is_empty = false;
                        self.byte_index = 0;
                    } else {
                        // If there is no more data, set is_empty to true
                        self.is_empty = true
                    }
                }
                Err(_) => {
                    error!("Problem refilling queue");
                    self.queue.clear();
                    self.is_empty = true;
                }
            }
        }
    }

    /// Returns Some(true) if next bit is set, false if clear, None if no data is available
    pub fn bit(&mut self) -> Option<bool> {
        // Clean the queue
        self.check_queue(1);
        // If we are not empty
        if !self.is_empty {
            // Get a bit (true = set, false = clear)
            let bit: bool = (self.queue[self.byte_index] & (MASK >> self.bit_index)) > 0;
            // Increment the bit index
            self.bit_index += 1;
            // If we are at the end of a byte, update the both indecies
            if self.bit_index > 7 {
                self.bit_index = 0;
                self.byte_index += 1;
            }
            // Return the bit
            Some(bit)
        } else {
            // Return None if we are empty
            None
        }
    }

    /// Returns n bits as Option Vec, None if not enough data
    pub fn bits(&mut self, n: usize) -> Option<Vec<bool>> {
        // Clean the queue
        self.check_queue(n);
        // Check if we have enough data to fulfill the request
        if self.queue.len() * 8 < n + self.byte_index * 8 + self.bit_index {
            // If not, return None
            None
        } else {
            // Otherwise build a vec of bits to return
            Some(
                self.into_iter()
                    .take(n)
                    .fold(Vec::with_capacity(n), |mut acc, bit| {
                        acc.push(bit);
                        acc
                    }),
            )
        }
    }

    /// Optimized to return Some<u8> from the bit stream, irregardless of the current position of the index
    pub fn byte(&mut self) -> Option<u8> {
        // Clean the queue
        self.check_queue(8);
        // Check if we have enough data to fulfill the request
        if self.queue.len() * 8 < 8 + self.byte_index * 8 + self.bit_index {
            // If not, return None
            None
        } else {
            // Otherwise build a byte from the next eight bits and return
            // When we are at bit index 0, just grab the next byte
            let byte: u8 = if self.bit_index == 0 {
                self.queue[self.byte_index]
            } else {
                // Otherwise we need to build the byte from the tail of one byte and the head of another
                (self.queue[self.byte_index] << self.bit_index)
                    | (self.queue[self.byte_index + 1] >> (8 - self.bit_index))
            };
            // In either case, we are one byte further into the queue
            self.byte_index += 1;
            // Return the valid byte
            Some(byte)
        }
    }

    /// Return  Some(Vec) of n bytes/u8s, if available
    pub fn bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        // Clean the queue
        self.check_queue(n * 8);
        // Check if we have enough data to fulfill the request
        if self.queue.len() * 8 < n * 8 + self.byte_index * 8 + self.bit_index {
            // If not, return None
            None
        } else {
            // Otherwise build a vec bytes from and return it
            Some((0..n).fold(Vec::with_capacity(n), |mut v, _| {
                v.push(self.byte().unwrap_or_default());
                v
            }))
        }
    }

    /// Optimized to return Some<u32> from the bit stream, irregardless of the current position of the index.
    /// n is the number of bits to be read to build the u32 value
    pub fn bint(&mut self, n: usize) -> Option<u32> {
        // Clean the queue
        self.check_queue(n);
        // Check if we have enough data to fulfill the request
        if self.queue.len() * 8 < n + self.byte_index * 8 + self.bit_index {
            // If not, return None
            None
        } else {
            // Otherwise take the requisite number of bits and build a u32 and return it
            Some(self.into_iter().take(n).fold(0, |mut acc, bit| {
                acc <<= 1;
                if bit {
                    acc += 1
                };
                acc
            }))
        }
    }

    /// Return the number of bytes left in the local queue (may be more in the buffered input!!!)
    pub fn len(&mut self) -> usize {
        // Clean the queue
        self.check_queue(0);
        // Return the number of unused bytes in the queue.
        self.queue.len() - self.byte_index
    }

    /// Determines if the data is exhausted
    pub fn is_empty(&mut self) -> bool {
        // Clean the queue
        self.check_queue(0);
        // (Check queue will determine if there is no more data)
        self.is_empty
    }
}

impl<R> Iterator for BitReader<R>
where
    R: Read,
{
    type Item = bool;
    fn next(&mut self) -> Option<bool> {
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
        assert_eq!(br.bit(), Some(true));
        assert_eq!(br.bit(), Some(false));
        assert_eq!(br.bit(), Some(false));
        assert_eq!(br.bit(), Some(false));
        assert_eq!(br.bit(), Some(false));
        assert_eq!(br.bit(), Some(false));
        assert_eq!(br.bit(), Some(false));
        assert_eq!(br.bit(), Some(true));
        assert_eq!(br.bit(), None);
    }

    #[test]
    fn iter_test() {
        let x = [0b10000001_u8, 0b0100_1000].as_slice();
        let mut br = BitReader::new(x);
        assert_eq!(br.next(), Some(true));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(true));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(true));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(true));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), Some(false));
        assert_eq!(br.next(), None);
    }
    #[test]
    fn bint_test() {
        let x = [0b00011011].as_slice();
        let mut br = BitReader::new(x);
        assert_eq!(br.bint(5), Some(3));
        assert_eq!(br.bint(5), None);
        assert_eq!(br.bint(1), Some(0));
        assert_eq!(br.bint(2), Some(3));
    }
    #[test]
    fn bits_test() {
        let x = [0b00011011].as_slice();
        let mut br = BitReader::new(x);
        assert_eq!(br.bits(3), Some(vec![false, false, false]));
        assert_eq!(br.bits(8), None);
        assert_eq!(br.bits(5), Some(vec![true, true, false, true, true]));
        assert_eq!(br.bits(3), None);
    }
    #[test]
    fn bytes_test() {
        let x = "Hello, world!".as_bytes();
        let mut br = BitReader::new(x);
        assert_eq!(br.bytes(1), Some("H".as_bytes().to_vec()));
        assert_eq!(br.bytes(3), Some("ell".as_bytes().to_vec()));
        assert_eq!(br.bytes(12), None);
        assert_eq!(br.bytes(9), Some("o, world!".as_bytes().to_vec()));
    }
    #[test]
    fn len_test() {
        let x = "Hello, world!".as_bytes();
        let mut br = BitReader::new(x);
        assert_eq!(br.is_empty(), false);
        assert_eq!(br.len(), 13);
        assert_eq!(br.bytes(1), Some("H".as_bytes().to_vec()));
        assert_eq!(br.len(), 12);
        assert_eq!(br.bytes(3), Some("ell".as_bytes().to_vec()));
        assert_eq!(br.len(), 9);
        assert_eq!(br.bytes(12), None);
        assert_eq!(br.len(), 9);
        assert_eq!(br.is_empty(), false);
        assert_eq!(br.bytes(9), Some("o, world!".as_bytes().to_vec()));
        assert_eq!(br.len(), 0);
        assert_eq!(br.is_empty(), true);
        println!("{:?}", br.bit());
        assert_eq!(br.is_empty(), true);
    }
}
