use log::error;

/// Creates a bitstream for output. This does not write the stream, but just packs bits
/// into the output. last_bits is the number of unused bits at the end of output, and is
/// always a number between 0 and 7 inclusive. BitPacker allows for parallel processing of
/// multiple blocks of data, which then must be written serially.
pub struct BitPacker {
    pub output: Vec<u8>,
    pub last_bits: u8,
    // queue holds bits temporarily until we can put them on the output efficiently
    queue: u64,
    // q_bits is the number of valid bits in the queue
    q_bits: u8,
}

impl BitPacker {
    /// Create a new BitPacker with an output buffer with the capacity specified in size.
    pub fn new(size: usize) -> Self {
        Self {
            output: Vec::with_capacity(size),
            last_bits: 0,
            queue: 0,
            q_bits: 0,
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

    /*
    NOTE: out24 takes a u32.  The 8 most significant bits of the word indicate how
    many of the least significant bits will be written. Those bits must be aligned to
    the least signficant bit. (The middle bits are masked out.)
    binary encoded data and puts it on the stream.

    It is primarily used to write odd size data.
    Eg 0000100_00000000_00000000_00000010 writes out 0010.
    */
    /// Writes 0-24 bits encoded with the number of bits to write in the most
    /// significant byte of a 32 bit word.
    pub fn out24(&mut self, data: u32) {
        let depth = (data >> 24) as u8; //get bit length by shifting out the 24 data bits
        self.queue <<= depth; //shift queue by bit length
        self.queue |= (data & (0xffffffff >> (32 - depth))) as u64; //add data portion to queue
        self.q_bits += depth; //update depth of queue bits
        self.write_stream();
    }

    /// Puts a 32 bit word of pre-packed binary encoded data on the stream.
    pub fn out32(&mut self, data: u32) {
        self.queue <<= 32; //shift queue by bit length
        self.queue |= data as u64; //add data portion to queue
        self.q_bits += 32; //update depth of queue bits
        self.write_stream();
    }

    /// Puts a 16 bit word  of pre-packed binary encoded data on the stream.
    pub fn out16(&mut self, data: u16) {
        self.queue <<= 16; //shift queue by bit length
        self.queue |= data as u64; //add data portion to queue
        self.q_bits += 16; //update depth of queue bits
        self.write_stream();
    }

    // out_8 is not used, but is here for completeness.
    // /// Puts an 8 bit word  of pre-packed binary encoded data on the stream.
    // pub fn out8(&mut self, data: u8) {
    //     self.queue <<= 8; //shift queue by bit length
    //     self.queue |= data as u64; //add data portion to queue
    //     self.q_bits += 8; //update depth of queue bits
    //     self.write_stream();
    // }

    /// Flushes the remaining bits (1-7) from the buffer, padding with 0s in the least
    /// signficant bits. Flush MUST be called before reading the output or data may be
    /// left in the internal queue.
    pub fn flush(&mut self) {
        self.last_bits = self.q_bits % 8;
        if self.q_bits > 0 {
            self.queue <<= 8 - self.q_bits; //pad the queue with zeros
            self.q_bits += 8 - self.q_bits;
            self.write_stream(); // write out all that is left
            if self.q_bits > 0 {
                error!("Stuff left in the BitPacker queue.");
            }
        }
    }

    /// Debugging function to return the number of bytes.bits output so far
    pub fn loc(&self) -> String {
        format! {"[{}.{}]",((self.output.len() * 8) + self.q_bits as usize)/8, ((self.output.len() * 8) + self.q_bits as usize)%8}
    }
}

#[cfg(test)]
mod test {
    use super::BitPacker;

    #[test]
    fn out16_test() {
        let mut bw = BitPacker::new(100);
        let data = 0b00100001_00100000;
        bw.out16(data);
        bw.flush();
        let out = bw.output;
        assert_eq!(out, "! ".as_bytes());
    }

    #[test]
    fn out24_and_loc_test() {
        let mut bw = BitPacker::new(100);
        let data = 0b00001000_00000000_00000000_00100001;
        bw.out24(data);
        bw.flush();
        let out = &bw.output;
        assert_eq!(out, "!".as_bytes());
        assert_eq!("[1.0]", &bw.loc());
        let data = 0b00011000_00000000_00000000_00000011;
        bw.out24(data);
        bw.flush();
        let out2 = &bw.output;
        assert_eq!(out2, &[33, 0, 0, 3]); // Note: '33' is data from previous call
        assert_eq!("[4.0]", &bw.loc());
    }

    #[test]
    fn out32_test() {
        let mut bw = BitPacker::new(100);
        let data = 0b00100001_00100000_00100001_00100000;
        bw.out32(data);
        bw.flush();
        let out = bw.output;
        assert_eq!(out, [33, 32, 33, 32]);
    }
}
