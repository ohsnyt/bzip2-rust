use crate::tools::crc::do_stream_crc;

/// Writes a bitstream for output. Takes the blocks packed by BitPacker and assembles them with
/// the stream header and footer, calculating the stream CRC as it processes the blocks.
pub struct BitWriter {
    /// Output buffer used to write the bitstream.
    output: Vec<u8>,
    /// Private queue to hold bits that are waiting to be put as bytes into the output buffer.
    queue: u64,
    /// Count of valid bits in the queue.
    q_bits: u8,

    /// Handle to the output stream
    writer: Box<dyn std::io::Write + std::marker::Sync + std::marker::Send>,
    /// Block size, needed to create the header.
    block_size: u8,
    /// Stream CRC, calculated from each block crc and added to the stream footer.
    stream_crc: u32,
}

impl BitWriter {
    /// Create a new Bitwriter with an output buffer of size specified. We need the block size. 
    /// to create the header. Use add_block() to add each block to the stream.
    pub fn new(filepath: &str, mut block_size: u8) -> Self {
        // Ensure that the block size is valid
        let result = std::fs::File::create(filepath);
        if block_size > 9 {
            block_size = 9;
        }
        // Open the output device for writing and initialize the struct
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
    fn push_header(&mut self) {
        // First write file stream header onto the stream
        let magic = "BZh".as_bytes();
        magic.iter().for_each(|&x| self.out8(x));
        self.out8(self.block_size + 0x30);
    }

    /// Add a block of data to the output. The block is assumed to be packed by BitPacker. "last" 
    /// indicates if the block is the last block in the file. "padding" indicates how many
    /// trailing zeros were added to the last byte of the block to make it a multiple of 8 bits.
    pub fn add_block(
        &mut self,
        last: bool,
        data: &[u8],
        padding: u8,
    ) -> Result<usize, std::io::Error> {
        // If this is the first block, write the header
        if self.stream_crc == 0 {
            self.push_header()
        };

        // Get the CRC from this block
        let block_crc = u32::from_be_bytes(data[6..10].try_into().unwrap());
        // Update the stream crc
        self.stream_crc = do_stream_crc(self.stream_crc, block_crc);

        // Write all the block data
        data.iter().for_each(|&x| self.out8(x));

        // Back up the queue to remove any padding on the last byte
        if padding > 0 {
            self.queue >>= padding as u64;
            self.q_bits -= padding
        }

        // If this is the last block, add the footer to the queue.
        if last {
            // First the stream footer magic, then the block_crc
            let magic = [0x17, 0x72, 0x45, 0x38, 0x50, 0x90];
            magic.iter().for_each(|&x| self.out8(x));
            // Write the stream crc
            self.out8((self.stream_crc >> 24) as u8);
            self.out8((self.stream_crc >> 16) as u8);
            self.out8((self.stream_crc >> 8) as u8);
            self.out8((self.stream_crc) as u8);

            // Now flush the queue
            self.flush();

            // And write out the remaining (flushed) data in the bitstream buffer.
            let result = self.writer.write(&self.output);

            // Drain what we wrote from the buffer
            if let Ok(written) = result {
                self.output.drain(..written);
            }
            return result;
        } else {
            // Write out the data in the bitstream buffer. The queue will carry over to the next block.
            let result = self.writer.write(&self.output);
            if result.is_ok() {
                self.output.drain(..result.as_ref().unwrap());
            }
            return result;
        }
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
        // Make sure the queue is empty enough to hold the data
        self.push_queue();
        self.queue <<= 8; //shift queue by one byte
        self.queue |= data as u64; //add the byte to queue
        self.q_bits += 8; //update depth of queue bits
    }

    /// Flushes the remaining bits (1-7) from the buffer, padding with 0s in the least
    /// signficant bits. Flush MUST be called before reading the output or data may be
    /// left in the internal queue.
    fn flush(&mut self) {
        // First push out all the full bytes
        while self.q_bits > 7 {
            let byte = (self.queue >> (self.q_bits - 8)) as u8;
            self.output.push(byte); //push the packed byte out
            self.q_bits -= 8; //adjust the count of bits left in the queue
        }
        // Then push out the remaining bits
        if self.q_bits > 0 {
            let mut byte = (self.queue & (0xff >> 8 - self.q_bits) as u64) as u8;
            byte <<= 8 - self.q_bits;
            self.output.push(byte); //push the packed byte out
            self.q_bits = 0; //adjust the count of bits left in the queue
        }
    }
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

    #[test]
    fn last_bits_test_1() {
        let mut bw = BitWriter::new("", 1);
        bw.out8(255);
        bw.out8(1);
        bw.out8(128);
        bw.out8(255);
        bw.out8(7<<5);
        bw.flush();
        let out = bw.output;
        assert_eq!(out, vec![255, 1, 128, 255, 224]);
    }

    #[test]
    fn out24_short_test() {
        let mut bw = BitWriter::new("", 100);
        bw.out8(255);
        bw.out8(6<<5);
        bw.flush();
        let out2 = &bw.output;
        assert_eq!(out2, &[0b1111_1111, 0b1100_0000]); // Note: '33' is data from previous call
    }
}
