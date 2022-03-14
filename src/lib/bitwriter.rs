/// Creates a bitstream for output. Although output is accessible at any time, it 
/// is best to call Flush before reading the "final" output.
pub struct BitWriter {
    pub output: Vec<u8>,
    queue: u64,
    q_bits: u8,
}

impl BitWriter {
    /// Called to create a new bitwriter
    pub fn new() -> Self {
        Self {
            output: Vec::new(),
            queue: 0,
            q_bits: 0,
        }
    }

    /// Internal bitstream write function common to all out.XX functions.
    /// (Leaves the queue dirty, but that should be okay)
    fn write_stream(&mut self) {
        while self.q_bits > 7 {
            let byte = (self.queue >> (self.q_bits - 8)) as u8;
            self.output.push(byte); //push the packed byte out
            self.q_bits -= 8; //adjust the count of bits left in the queue
        }
    }
    /// Takes a 32 bit word in the format of a u8 as bit count in the most
    /// significant bits of the word plus 0-24 bits of right aligned
    /// binary encoded data and puts it on the stream.
    /// Primarily used to handle odd size data.
    /// eg 0000100_00000000_00000000_00000010, which writes out 0010.
    pub fn out24(&mut self, data: u32) {
        let depth = (data >> 24) as u8; //get bit length
        self.queue <<= depth; //shift queue by bit length
        self.queue |= (data & 0x00ffffff) as u64; //add data portion to queue
        self.q_bits += depth; //update depth of queue bits
        self.write_stream();
    }

    /// Takes a 32 bit word of pre-packed binary encoded data and puts it on the stream.
    pub fn out32(&mut self, data: u32) {
        self.queue <<= 32; //shift queue by bit length
        self.queue |= data as u64; //add data portion to queue
        self.q_bits += 32; //update depth of queue bits
        self.write_stream();
    }

    /// Takes a 16 bit word  of pre-packed binary encoded data and puts it on the stream.
    pub fn out16(&mut self, data: u16) {
        self.queue <<= 16; //shift queue by bit length
        self.queue |= data as u64; //add data portion to queue
        self.q_bits += 16; //update depth of queue bits
        self.write_stream();
    }

    /// Takes an 8 bit word  of pre-packed binary encoded data and puts it on the stream.
    pub fn out8(&mut self, data: u8) {
        self.queue <<= 8; //shift queue by bit length
        self.queue |= data as u64; //add data portion to queue
        self.q_bits += 8; //update depth of queue bits
        self.write_stream();
    }

    /// Flushes the remaining bits (1-7) from the buffer, padding with 0s in the least 
    /// signficant bits
    pub fn flush(&mut self) {
        if self.q_bits > 0 {
            self.queue <<= 8 - self.q_bits; //pad the queue with zeros
            self.output.push(self.queue as u8); //push the packed byte out
            self.queue = 0; //clear the queue
            self.q_bits = 0; //clear the queue bit counter
        }
    }
}
