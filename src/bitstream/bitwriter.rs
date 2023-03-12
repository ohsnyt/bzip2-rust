use simplelog::error;

use crate::tools::crc::do_stream_crc;

/// Writes a bitstream for output. This is very similar to the BitPacker, except it
/// writes to a output file/stream (through a buffer).
struct BitWriter {
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
        let result = std::fs::File::open(filepath);
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

    /// Write stream header to output buffer.
    fn write_header(&self) {
        // First write file stream header onto the stream
        let magic = "BZh".as_bytes();
        magic.iter().for_each(|&x| self.out8(x));
        self.out8(self.block_size + 0x30);
    }

    /// Add a block of data to the output.
    pub fn add_block(
        &self,
        first: bool,
        last: bool,
        crc: u32,
        data: &[u8],
    ) -> Result<usize, std::io::Error> {
        if first {
            self.write_header()
        };
        data.iter().for_each(|&x| self.out8(x));
        self.stream_crc = do_stream_crc(self.stream_crc, crc);
        if last {
            self.write_footer()
        } else {
            Ok(())
        };
        // Write out the data in the bitstream buffer.
        let written = self.writer.write(&self.output);
        if written.is_ok() {
            self.output.drain(..written.unwrap());
        }
        written
    }

    /// Write stream footer to output buffer and flush buffer.
    fn write_footer(&self) -> Result<(), std::io::Error> {
        // At the last block, write the stream footer magic and  block_crc and flush the output buffer
        let magic = [0x17, 0x72, 0x45, 0x38, 0x50, 0x90];
        magic.iter().for_each(|&x| self.out8(x));
        self.out32(self.stream_crc);
        self.flush();

        // Write out the data in the bitstream buffer.
        self.writer.write_all(&self.output)
    }
    /// Flushes the remaining bits (1-7) from the buffer, padding with 0s in the least
    /// signficant bits. Mst be called prior to closing the output file/stream.
    fn flush(&mut self) {
        if self.q_bits > 0 {
            self.queue <<= 8 - self.q_bits; //pad the queue with zeros
            self.q_bits += 8 - self.q_bits;
            self.write_stream(); // write out all that is left
            if self.q_bits > 0 {
                error!("Stuff left in the BitWriter queue.");
            }
        }
    }
    /// Internal bitstream write function common to all out.XX functions.
    fn write_stream(&mut self) {
        while self.q_bits > 7 {
            let byte = (self.queue >> (self.q_bits - 8)) as u8;
            self.output.push(byte); //push the packed byte out
            self.q_bits -= 8; //adjust the count of bits left in the queue
        }
    }

    /// Puts a 32 bit word of pre-packed binary encoded data on the stream.
    pub fn out32(&mut self, data: u32) {
        self.queue <<= 32; //shift queue by bit length
        self.queue |= data as u64; //add data portion to queue
        self.q_bits += 32; //update depth of queue bits
        self.write_stream();
    }

    /// Puts an 8 bit word  of pre-packed binary encoded data on the stream.
    pub fn out8(&mut self, data: u8) {
        self.queue <<= 8; //shift queue by bit length
        self.queue |= data as u64; //add data portion to queue
        self.q_bits += 8; //update depth of queue bits
        self.write_stream();
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

    #[test]
    fn out32_test() {
        let mut bw = BitWriter::new("", 1);
        let data = 0b00100001_00100000_00100001_00100000;
        bw.out32(data);
        bw.flush();
        let out = bw.output;
        assert_eq!(out, [33, 32, 33, 32]);
    }
}
