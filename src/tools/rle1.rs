use super::crc::do_crc;

const MAX_RUN: usize = 256 + 4;

/// Iteratable struct that will return a block with a max size of block_size bytes
/// encoded using BZIP2 RLE 1 style encoding
pub struct RLE1Block<R> {
    source: R,
    block_size: usize,
    buffer: Vec<u8>,
    buffer_index: usize,
    data_gone: bool,
    pub block_crc: u32,
}

impl<R: std::io::Read> RLE1Block<R> {
    pub fn new(source: R, block_size: usize) -> Self {
        RLE1Block {
            source,
            block_size,
            buffer: Vec::with_capacity(block_size + 264),
            buffer_index: 0,
            data_gone: false,
            block_crc: 0,
        }
    }

    /// Check (and refill) a low buffer - true if we have data, false if there is no more.
    /// Refill when there is less than 264 bytes. We want to keep that many for comparision in case
    /// the run happens over the end of our last read.
    fn refill_buffer(&mut self) -> bool {
        // If we have less than 264 bytes of data in our buffer, go try to get more
        if self.data_gone || self.buffer.len() - self.buffer_index < MAX_RUN {
            // First, removed data we have already processed
            self.buffer.drain(..self.buffer_index);
            self.buffer_index = 0;
            // Then get more data
            let mut temp_buffer = vec![0; self.block_size];
            let received = self
                .source
                .read(&mut temp_buffer)
                .expect("Unble to read source data");
            // Append the new data to our buffer and adjust our counter for how much we have left.
            temp_buffer.truncate(received);
            self.buffer.append(&mut temp_buffer);
            // If we read all the data, remember that it is gone.
            if received < self.block_size {
                self.data_gone = true;
                return false;
            }
        }
        true
    }

    /// Encode runs of for our more identical bytes, pre-BWT. Returns block_crc and the RLE1 data.
    fn get_block(&mut self) -> (u32, Vec<u8>) {
        // Since we only get here by the iterator, we can assume we have data in our buffer.
        // Reserve space for the output, allowing an extra byte in case we end in a run and need to add a count
        let mut out: Vec<u8> = Vec::with_capacity(self.block_size + 1);
        // Pushing a byte at a time to the output is slower than extending by chunks, therefore initialize
        // variable to remember where each chunk of non-runs starts so we can append data in chunks.
        let mut start: usize = 0;
        // Set a counter to the amount of unproccessed data is in the buffer, checking for no data
        let mut remaining = self.buffer.len() - 1;

        // Now process the input until we reach our desired amount (including what is in a pending chunk)
        while out.len() + (self.buffer_index - start) < self.block_size {
            match remaining {
                // In the case that we need 0 bytes, all the data should be processed. We should only
                // get here when the data ends with a run of duplicates. Clean up and return.
                0 => {
                    // Empty the buffer and reset the index
                    self.buffer.clear();
                    self.buffer_index = 0;
                    return (self.block_crc, out);
                }
                // In the case that we need 1, 2 or 3 bytes, we don't need to look for runs.
                1..=3 => {
                    // Get to the end of the buffer
                    self.buffer_index += remaining + 1;
                    out.extend_from_slice(&self.buffer[start..self.buffer_index]);
                    self.block_crc = do_crc(self.block_crc, &self.buffer[start..self.buffer_index]);
                    self.buffer.drain(..self.buffer_index);
                    self.buffer_index = 0;
                    return (self.block_crc, out);
                }
                _ => {
                    // If the buffer is low, first copy out what we have processed and then go refill it.
                    if remaining < MAX_RUN && !self.data_gone {
                        self.block_crc =
                            do_crc(self.block_crc, &self.buffer[start..self.buffer_index]);
                        out.extend_from_slice(&self.buffer[start..self.buffer_index]);
                        self.buffer.drain(..self.buffer_index);
                        self.buffer_index = 0;
                        self.refill_buffer();
                        remaining = self.buffer.len();
                        start = 0;
                    }
                    // Then look for a run of 4 bytes, adjusting the remaining counter as appropriate
                    if self.buffer[self.buffer_index] == self.buffer[self.buffer_index + 1] {
                        if self.buffer[self.buffer_index] == self.buffer[self.buffer_index + 2] {
                            if self.buffer[self.buffer_index] == self.buffer[self.buffer_index + 3]
                            {
                                // Get the count of duplicates following (0-255)
                                let dups = self.count_dups();
                                // reset the buffer index to the end of the run of 4
                                self.buffer_index += 4 + dups as usize;
                                remaining -= remaining.min(4 + dups as usize);
                                // If they are, calculate the CRC from the last start until the end of the duplicates
                                self.block_crc = do_crc(
                                    self.block_crc,
                                    &self.buffer[start..(self.buffer_index as usize)],
                                );
                                out.extend_from_slice(
                                    &self.buffer[start..self.buffer_index - dups as usize],
                                );
                                // Write the duplicate count
                                out.push(dups);
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
            }
        }
        // We filled the block (or can with the pending append), so return it
        self.block_crc = do_crc(self.block_crc, &self.buffer[start..self.buffer_index]);
        out.extend_from_slice(&self.buffer[start..self.buffer_index]);
        self.buffer.drain(..self.buffer_index);
        self.buffer_index = 0;
        (self.block_crc, out)
    }

    /// Helper function for rel1_encode to count how many duplicate bytes occur (0-255).
    fn count_dups(&self) -> u8 {
        // self.buffer_index is the position of the first of four identical bytes. We need to count
        // identical bytes *after* the fourth. If position returns None, then return
        // the number of bytes to the end of the data we have taken.
        let compare = self.buffer[self.buffer_index];
        self.buffer
            .iter()
            .skip(self.buffer_index + 4)
            .take(255)
            .position(|&x| x != compare)
            .unwrap_or(self.buffer.len() - self.buffer_index - 4) as u8
    }
}

impl<R> Iterator for RLE1Block<R>
where
    R: std::io::Read,
{
    type Item = (u32, Vec<u8>);
    fn next(&mut self) -> Option<(u32, Vec<u8>)> {
        // First make sure the buffer is full.
        self.refill_buffer();

        // If there is no data to process, return None (Nothing to read and an empty buffer).
        if self.data_gone && self.buffer.len() == 0 {
            return None;
        }
        // Otherwise go process a block (size set by block_size) of data and return the block
        Some(self.get_block())
    }
}

/*
Logic: This is similar to the encoding. Start looking for a sequence of 4 identical bytes.
When you find them, get the next byte, which is a count of how many more such bytes are needed.
First output everthing from the start until the end of the sequence we found (not counting the
count byte) followed by a vec created with the repeating byte we want to insert. Since we want
to get past the 4 bytes plus the counter that we found, put 5 into a jump_past_search variable
and loop until that decrements to zero.
Perhaps I should have use the while idx strategy rather than a for loop, but...
*/
/// Unencodes runs of four or more characters from the RLE1 phase
pub fn rle1_decode(v: &[u8]) -> Vec<u8> {
    let mut start: usize = 0;
    let mut jump_past_search = 0;
    let mut out = vec![];
    for i in 0..v.len() - 4 {
        if jump_past_search > 0 {
            jump_past_search -= 1;
        } else if v[i] == v[i + 1] && v[i] == v[i + 2] && v[i] == v[i + 3] {
            out.extend_from_slice(&v[start..i + 4]);
            let tmp = vec![v[i]; v[i + 4].into()];
            out.extend(tmp);
            jump_past_search = 5;
            start = i + jump_past_search;
        }
    }
    out.extend_from_slice(&v[start..v.len()]);
    out
}
