use super::crc::do_crc;

const MAX_RUN: usize = 256 + 4;

/// Iteratable struct that will return a block with a max size of block_size bytes
/// encoded using BZIP2 RLE 1 style encoding
pub struct RLE1Block<R> {
    source: R,
    block_size: usize,
    buffer: Vec<u8>,
    buffer_index: usize,
    crc: u32,
    pub bytes_used: usize,
}

impl<R: std::io::Read> RLE1Block<R> {
    pub fn new(source: R, block_size: usize) -> Self {
        RLE1Block {
            source,
            block_size,
            buffer: Vec::with_capacity(block_size + 264),
            buffer_index: 0,
            crc: 0,
            bytes_used: 0,
        }
    }

    /// Check (and refill) buffer - true if we have data, false if there is no more.
    /// Refill when there is less than 264 bytes. We want to keep that many for comparision in case
    /// the run happens over the end of our last read.
    fn have_data(&mut self) -> bool {
        // If we have less than 264 bytes of data in our buffer, go try to get more
        if self.buffer.len() - self.buffer_index < MAX_RUN {
            // First, removed data we have already processed
            self.buffer.drain(..self.buffer_index);
            self.bytes_used += self.buffer_index;
            self.buffer_index = 0;
            // Then get more data
            let mut temp_buffer = vec![0; self.block_size];
            let received = self
                .source
                .read(&mut temp_buffer)
                .expect("Unble to read source data");
            // If we failed to find any more data, return false
            if received == 0 {
                return false;
            } else {
                // Append the new data to our buffer and adjust our counter for how much we have left.
                temp_buffer.truncate(received);
                self.buffer.append(&mut temp_buffer);
            }
        }
        true
    }

    /// Encode runs of for our more identical bytes, pre-BWT. Returns crc of uncompressed data and the RLE1 data.
    fn rle_encode(&mut self, free_space: usize) -> (u32, Vec<u8>) {
        // Start by ensuring we have data in our buffer.
        let mut found_data = self.have_data();
        // And clean up the data buffer of any used data, resetting the index to zero.
        self.buffer.drain(..self.buffer_index);
        self.bytes_used += self.buffer_index;
        self.buffer_index = 0;
        // No need to check for runs if the data is less than 4 bytes or we need less than 4 bytes
        if self.buffer.len() < 4 || free_space < 4 {
            let offset = self.buffer.len().min(free_space);
            self.bytes_used += offset;
            self.crc = do_crc(self.crc, &self.buffer[..offset]);
            return (self.crc, self.buffer[..offset].to_vec());
        }
        // Reserve space for the output, allowing an extra byte in case we end in a run and need to add a count
        let mut out = Vec::with_capacity(free_space + 1);
        // Pushing a byte at a time to the output is slower than extending by chunks, therefore initialize
        // variable to remember where each chunk of non-runs starts so we can append them in chunks.
        let mut start: usize = 0;
        // And remember how much data is in the buffer
        let mut remaining = self.buffer.len();

        // As long as we have data and have not processed our limit, process data for rle1 sequences
        while (remaining > 3) && (out.len() + self.buffer_index - start < free_space) {
            // Make sure we have enough in our buffer to finish a run if we find one toward the end, but
            // only try to get more data if there is more data to get
            if found_data && remaining < MAX_RUN {
                // Append buffer from start to index to the output, then go grab more data
                self.crc = do_crc(self.crc, &self.buffer[start..=self.buffer_index]);
                out.extend_from_slice(&self.buffer[start..=self.buffer_index]);
                self.buffer.drain(..self.buffer_index);
                self.bytes_used += self.buffer_index;
                self.buffer_index = 0;
                found_data = self.have_data();
                remaining = self.buffer.len();
                start = 0;
            }
            // Look at the next 4 bytes to see if they are identical, testing the distant case first
            if self.buffer[self.buffer_index] == self.buffer[self.buffer_index + 1] {
                if self.buffer[self.buffer_index] == self.buffer[self.buffer_index + 2] {
                    if self.buffer[self.buffer_index] == self.buffer[self.buffer_index + 3] {
                        // If they are, write out everything from the last start until the end of the run of 4
                        self.crc = do_crc(self.crc, &self.buffer[start..=self.buffer_index + 3]);
                        out.extend_from_slice(&self.buffer[start..=self.buffer_index + 3]);
                        // reset the buffer index
                        self.buffer_index += 3;
                        remaining -= 3;
                        // Get the count of duplicates following (0-255)
                        let dups = self.count_dups();
                        // Write that out
                        self.crc = do_crc(
                            self.crc,
                            &vec![self.buffer[self.buffer_index - 1]; dups as usize],
                        );
                        out.push(dups);
                        // Move the index past the duplicates we counted
                        self.buffer_index += dups as usize;
                        remaining -= dups as usize;
                        // Reset start to this new chunk
                        start = self.buffer_index;
                    } else {
                        // Otherwise increment the index past our search
                        self.buffer_index += 3;
                        remaining -= 3;
                        continue;
                    }
                } else {
                    // Otherwise increment the index past our search
                    self.buffer_index += 2;
                    remaining -= 2;
                    continue;
                }
            } else {
                // Otherwise increment the index past our search
                self.buffer_index += 1;
                remaining -= 1
            }
        }
        // Write out any pending data.
        self.crc = do_crc(self.crc, &self.buffer[start..self.buffer_index]);
        out.extend_from_slice(&self.buffer[start..self.buffer_index]);
        // Report how much of the input was processed and return the output
        self.bytes_used += self.buffer.len() - remaining;
        return (self.crc, out);
    }

    /// Helper function for rel1_encode to count how many duplicate bytes occur (0-255).
    fn count_dups(&self) -> u8 {
        // self.buffer_index is the position of the last of four identical bytes. We need to count
        // identical bytes *after* that position. If position returns None, then return
        // the number of bytes to the end of the data we have taken.
        let compare = self.buffer[self.buffer_index];
        self.buffer
            .iter()
            .skip(self.buffer_index)
            .take(255)
            .position(|&x| x != compare)
            .unwrap_or(self.buffer.len() - self.buffer_index) as u8
    }
}

impl<R: std::io::Read> Iterator for RLE1Block<R> {
    type Item = (u32, Vec<u8>);
    fn next(&mut self) -> Option<Self::Item> {
        // If there is no more data to read, return None
        if !self.have_data() {
            return None;
        }
        Some(self.rle_encode(self.block_size))
    }
}
