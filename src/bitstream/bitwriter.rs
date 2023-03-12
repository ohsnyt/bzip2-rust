use simplelog::error;

use crate::tools::crc::do_stream_crc;

/// Writes a bitstream for output. This is very similar to the BitPacker, except it
/// writes to a output file/stream (through a buffer).
pub struct BitWriter {
    output: Vec<u8>,
    queue: u64,
    q_bits: u8,

    writer: Box<dyn std::io::Write>,
    block_size: u8,
    stream_crc: u32,
}

impl BitWriter {
    /// Create a new Bitwriter with an output buffer of size specified. Suggest the
    /// size be set to the block size. Call flush() to flush the bit queue to the buffer
    /// before closing the output file.
    pub fn new(filepath: &str, mut block_size: u8) -> Self {
        let result = std::fs::File::create(filepath);
        if block_size > 9 {
            block_size = 9;
        }
        Self {
            writer: match result {
                Ok(file) => Box::new(file),
                Err(_) => Box::new(std::io::stdout()),
            },
            output: Vec::with_capacity(block_size as usize * 100000),
            queue: 0,
            q_bits: 0,
            block_size,
            stream_crc: 0,
        }
    }

    /// Push the stream header to output buffer.
    fn push_header(&self) {
        // First write file stream header onto the stream
        let magic = "BZh".as_bytes();
        magic.iter().for_each(|&x| self.out8(x));
        self.out8(self.block_size + 0x30);
    }

    /// Add a block of data to the output.
    pub fn add_block(&self, last: bool, data: &[u8], last_bits: u8) -> Result<usize, std::io::Error> {
        // If this is the first block, write the header
        if self.stream_crc == 0 {
            self.push_header()
        };
        // Get the CRC from this block
        let block_crc = u32::from_be_bytes(data[6..10].try_into().unwrap());
        // Update the stream crc
        self.stream_crc = do_stream_crc(self.stream_crc, block_crc);
        // Write all the data except the last byte
        data.iter().take(data.len()-1).for_each(|&x| self.out8(x));
        // Write the good bits from the last byte
         self.last_bits(*data.last().unwrap(), last_bits);
        // Write out the data in the bitstream buffer.
        let mut result = self.writer.write(&self.output);
        if result.is_ok() {
            self.output.drain(..result.unwrap());
        } else {
            return result;
        }
        // If this is the last block, write the footer
        if last {
            // At the last block, write the stream footer magic and  block_crc and flush the output buffer
            let magic = [0x17, 0x72, 0x45, 0x38, 0x50, 0x90];
            magic.iter().for_each(|&x| self.out8(x));
            // Write the stream crc
            self.out8((self.stream_crc >> 24) as u8);
            self.out8((self.stream_crc >> 16) as u8);
            self.out8((self.stream_crc >> 8) as u8);
            self.out8((self.stream_crc >> 0) as u8);
            self.flush();

            // Write out the data in the bitstream buffer.
            let mut result = self.writer.write(&self.output);
            if result.is_ok() {
                self.output.drain(..result.unwrap());
                if self.output.is_empty() {
                    return Ok(0);
                }
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to write bzip2 stream footer.",
                ));
            }
        }
        Ok(result.unwrap())
    }

    /// Internal bitstream write function common to all out.XX functions.
    fn push_queue(&mut self) {
        // If the queue has less than 8 bits left, write all full bytes to the output buffer.
        if self.q_bits > 56 {
            while self.q_bits > 7 {
                let byte = (self.queue >> (self.q_bits - 8)) as u8;
                self.output.push(byte); //push the packed byte out
                self.q_bits -= 8; //adjust the count of bits left in the queue
            }
        }
    }

    /// Put a byte of pre-packed binary encoded data on the stream.
    fn out8(&mut self, data: u8) {
        self.queue <<= 8; //shift queue by one byte
        self.queue |= data as u64; //add the byte to queue
        self.q_bits += 8; //update depth of queue bits
        self.push_queue();
    }

    /// Puts a partial byte of pre-packed binary encoded data on the stream.
    fn last_bits(&mut self, data: u8, mut last_bits: u8) {
        self.queue <<= last_bits.max(8); //bit shift queue by up to one byte
        self.queue |= (data >> 8 - last_bits) as u64; //bit shift and add the data to queue
        self.q_bits += last_bits; //update depth of queue bits
        self.push_queue();
    }

    /// Flushes the remaining bits (1-7) from the buffer, padding with 0s in the least
    /// signficant bits. Flush MUST be called before reading the output or data may be
    /// left in the internal queue.
    fn flush(&mut self) {
        if self.q_bits > 0 {
            self.queue <<= 8 - self.q_bits; //pad the queue with zeros
            self.q_bits += 8 - self.q_bits;
            self.push_queue(); // write out all that is left
            if self.q_bits > 0 {
                error!("Stuff left in the BitPacker queue.");
            }
        }
    }

    // /// Debugging function to return the number of bytes.bits output so far. Used in tests.
    // pub fn loc(&self) -> String {
    //     format! {"[{}.{}]",((self.output.len() * 8) + self.q_bits as usize)/8, ((self.output.len() * 8) + self.q_bits as usize)%8}
    // }
}

#[cfg(test)]
mod test {
    use super::BitWriter;

    #[test]
    fn out8_test() {
        let mut bw = BitWriter::new("", 1);
        let data = 'x' as u8;
        bw.out8(data);
        bw.flush();
        let out = bw.output;
        assert_eq!(out, "x".as_bytes());
    }
    fn last_bits_test() {
        let mut bw = BitWriter::new("", 1);
        let data = 0xFF as u8;
        bw.out8(data);
        let data = 0x6 as u8;
        bw.last_bits(data, 3);
        bw.flush();
        let out = bw.output;
        assert_eq!(out, vec![0xFF, 0xE0]);
    }
}
