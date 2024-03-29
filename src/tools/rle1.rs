//! Perform the first run-length-encoding (RLE1) transform for the BZIP2 file formate.
//!
//! The run-length-encoding compresses runs of 4-256 identical bytes in the initial data. If a run is longer than 256 bytes, it may be
//! encoded as two or more runs. (258 identical bytes will be see as one run of 256 bytes and two identical bytes (which is not long enough
//! to be encoded as another run.)
//!
//! The RLE1 phase happens **before** determining what data will be placed into each block. Because of this,
//! the RLE1 phase is build as an iterator over the raw data. You must instantiate a RLE1Block struct before
//! you can use it.
//!
//! Usage is:
//! ```
//! let mut rle1 = RLE1Block::new(data, block_size);
//! ```
//! Where:
//! - data: The data to be run-length encoded.
//! - block_size: The actual size of each block that will be created.
//!
//! To get a block of data, you must iterate or call .next() on the struct. For example:
//! ```
//! let (crc, block, last_block) = rle1.next().unwrap();
//! ```
//! This returns the crc value for the block, the block of data, and a boolean indicating if the block is the last block.
//!
//! 

use super::crc::do_crc;

const MAX_RUN: usize = 256 + 4;

/// Iteratable struct that will return a block with a max size of block_size bytes
/// encoded using BZIP2 RLE 1 style encoding
pub struct RLE1Block<R>
where
    R: std::io::Read + std::marker::Sync + std::marker::Send,
{
    source: R,
    block_size: usize,
    buffer: Vec<u8>,
    buffer_cursor: usize,
    data_gone: bool,
    pub block_crc: u32,
}

impl<R> RLE1Block<R>
where
    R: std::io::Read + std::marker::Sync + std::marker::Send,
{
    pub fn new(source: R, block_size: usize) -> Self {
        RLE1Block {
            source,
            block_size,
            buffer: Vec::with_capacity(block_size + 264),
            buffer_cursor: 0,
            data_gone: false,
            block_crc: 0,
        }
    }

    /// Check (and refill) a low buffer - true if we have data, false if there is no more.
    /// Refill when there is less than 256 bytes. We want to keep that many for comparision in case
    /// the run happens over the end of our last read.
    fn refill_buffer(&mut self) -> bool {
        // If we have less than 256 bytes of data in our buffer, go try to get more
        if self.data_gone || self.buffer.len() - self.buffer_cursor < MAX_RUN {
            // First, removed data we have already processed
            self.buffer.drain(..self.buffer_cursor);
            self.buffer_cursor = 0;
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

    /// Encode runs of for our more identical bytes, pre-BWT. Returns a crc of the original data used,
    ///  the RLE1 data, and a bool set to true if this is the last block.
    fn get_block(&mut self) -> (u32, Vec<u8>, bool) {
        /*
        This is optimized for speed. It scans the input for runs of 4 identical bytes. A run can be anywhere
        from 0-251 identical bytes after the run. This means our buffer should be at least 256 bytes long so
        we can scan to the end of the longest run we can encode.

        We must build a crc of the input data (not the RLE1 data). Since we only add data to the output
        whenever we encounter a run (or reach the end of the block, or have to refill the buffer), we will
        only compute the current crc at those times.
        */

        // Since we only get here by the iterator, we can assume we have data in our buffer.
        // Reserve space for the output, allowing an extra byte in case we end in a run and need to add a count
        let mut out: Vec<u8> = Vec::with_capacity(self.block_size + 1);
        // Pushing a byte at a time to the output is slower than extending by chunks, therefore initialize
        // variable to remember where each chunk of non-runs starts so we can append data in chunks.
        let mut start: usize = 0;
        // Set a counter to the amount of unproccessed data in the buffer, checking for no data
        let mut remaining = self.buffer.len();

        // Now process the input until we reach our desired amount (including what is in a pending chunk)
        while out.len() + (self.buffer_cursor - start) < self.block_size {
            // Check how many bytes are remaining in our buffer
            match remaining {
                // In the case that we have 0 bytes left, all the data should be processed. We should only
                // get here when the data ends with a run of duplicates. Clean up and return.
                0 => {
                    // Empty the buffer and reset the cursor
                    self.buffer.clear();
                    self.buffer_cursor = 0;
                    return (
                        self.block_crc,
                        out,
                        self.data_gone && self.buffer.is_empty(),
                    );
                }
                // In the case that we have only 1, 2 or 3 bytes left, we don't need to look for runs.
                1..=3 => {
                    // Get to the end of the buffer
                    self.buffer_cursor = self.buffer.len();
                    out.extend_from_slice(&self.buffer[start..self.buffer_cursor]);
                    self.block_crc =
                        do_crc(self.block_crc, &self.buffer[start..self.buffer_cursor]);
                    self.buffer.drain(..self.buffer_cursor);
                    self.buffer_cursor = 0;
                    return (
                        self.block_crc,
                        out,
                        self.data_gone && self.buffer.is_empty(),
                    );
                }
                // Otherwise we still need to look for runs.
                _ => {
                    // If the buffer is low, first copy out what we have processed and then go refill it.
                    if remaining < MAX_RUN && !self.data_gone {
                        // First shift the cursor back one. (If we get here after writing dups, we have
                        //  advanced it one past where we have last checked.)
                        self.buffer_cursor -= 1;
                        self.block_crc =
                            do_crc(self.block_crc, &self.buffer[start..self.buffer_cursor]);
                        out.extend_from_slice(&self.buffer[start..self.buffer_cursor]);
                        self.buffer.drain(..self.buffer_cursor);
                        self.buffer_cursor = 0;
                        self.refill_buffer();
                        remaining = self.buffer.len();
                        start = 0;
                    }
                    // Check to make sure we never begin at index 0 of the buffer.
                    if self.buffer_cursor == 0 {
                        self.buffer_cursor += 1;
                        remaining -= 1;
                    }

                    // Then look for a run of 4 bytes, adjusting the remaining counter as appropriate
                    // Look for a run of 4 identical bytes by first looking for two that are two bytes apart.
                    if self.buffer[self.buffer_cursor] != self.buffer[self.buffer_cursor + 2] {
                        self.buffer_cursor += 2;
                        remaining -= 2;
                        continue;
                    } // Found a set, now check the byte between and fail if it's not the same.
                    if self.buffer[self.buffer_cursor] != self.buffer[self.buffer_cursor + 1] {
                        self.buffer_cursor += 2;
                        remaining -= 2;
                        continue;
                    }
                    // Middle byte is okay, so check the byte before and after, and fail if both tests show misses.
                    if (self.buffer[self.buffer_cursor] != self.buffer[self.buffer_cursor - 1])
                        && (self.buffer[self.buffer_cursor] != self.buffer[self.buffer_cursor + 3])
                    {
                        self.buffer_cursor += 2;
                        remaining -= 2;
                        continue;
                    }
                    // We are now at a run of 4. If the byte before was the good one, move the cursor back one byte.
                    if self.buffer[self.buffer_cursor] == self.buffer[self.buffer_cursor - 1] {
                        self.buffer_cursor -= 1;
                    }
                    // We are now at a run of 4
                    {
                        // Get the count of duplicates following (0-251)
                        let dups = self.count_dups();
                        // reset the buffer cursor to the end of the run of 4
                        self.buffer_cursor += 4 + dups as usize;
                        // If they are, calculate the CRC from the last start until the end of the duplicates
                        self.block_crc = do_crc(
                            self.block_crc,
                            &self.buffer[start..(self.buffer_cursor as usize)],
                        );
                        out.extend_from_slice(
                            &self.buffer[start..self.buffer_cursor - dups as usize],
                        );
                        // Write the duplicate count
                        out.push(dups);
                        // Reset start to this new chunk
                        start = self.buffer_cursor;
                        // Move the cursor one past the start position
                        self.buffer_cursor += 1;
                        // Update the remaining counter
                        remaining = self.buffer.len() - start;
                    }
                }
            }
        }
        // We filled the block (or can with the pending append), so return it
        self.block_crc = do_crc(self.block_crc, &self.buffer[start..self.buffer_cursor]);
        out.extend_from_slice(&self.buffer[start..self.buffer_cursor]);
        self.buffer.drain(..self.buffer_cursor);
        self.buffer_cursor = 0;
        // Return the output
        (
            self.block_crc,
            out,
            self.data_gone && self.buffer.is_empty(),
        )
    }

    /// Helper function for rel1_encode to count how many duplicate bytes occur (0-251).
    fn count_dups(&self) -> u8 {
        // Test if the byte after the run is the same as the bytes in the run. If not, return 0
        if self.buffer[self.buffer_cursor] != self.buffer[self.buffer_cursor + 4] {
            return 0;
        }
        // self.buffer_cursor is the position of the first of four identical bytes. We need to count
        // identical bytes *after* the fourth. If position returns None, then return 256 or the data
        // remaining, if less.
        let compare = self.buffer[self.buffer_cursor];
        self.buffer
            .iter()
            .skip(self.buffer_cursor + 4)
            .take(251)
            .position(|&x| x != compare)
            .unwrap_or_else(|| 251.min(self.buffer.len() - (self.buffer_cursor + 4))) as u8
    }
}

/// Iterator for RLE1 encoding.
impl<R> Iterator for RLE1Block<R>
where
    R: std::io::Read + std::marker::Sync + std::marker::Send,
{
    type Item = (u32, Vec<u8>, bool);
    fn next(&mut self) -> Option<(u32, Vec<u8>, bool)> {
        // If there is no data to process, return None (Nothing to read and an empty buffer).
        if self.data_gone && self.buffer.is_empty() {
            return None;
        }

        // Otherwise, make sure the buffer is full.
        self.refill_buffer();
        // And clear the block crc value.
        self.block_crc = 0;

        // Then go process a block (size set by block_size) of data and return the block
        Some(self.get_block())
    }
}

/// Unencodes runs of four or more characters from the RLE1 phase
pub fn rle1_decode(rle1: &[u8]) -> Vec<u8> {
    /*
    Logic: This is similar to the encoding. Start looking for a sequence of 4 identical bytes.
    When you find them, get the next byte, which is a count of how many more such bytes are needed.

    First output everthing from the start until the end of the sequence we found (not counting the
    count byte) followed by a vec created with the repeating byte we want to insert.
    */

    // Initialize cursors for moving through a slice of the rle1 data
    let mut start = 0;
    let mut cursor = 1_usize;
    // Initialize the output vec with 120% capacity of the input, which should cover most cases.
    let mut out = Vec::with_capacity(rle1.len() * 5 / 4);

    // Process the RLE1 data.
    while cursor < rle1.len() - 4 {
        // Look for a run of 4 identical bytes by first looking for two that are two bytes apart.
        if rle1[cursor] != rle1[cursor + 2] {
            cursor += 2;
            continue;
        } // Found a set, now check the byte between and fail if it's not the same.
        if rle1[cursor] != rle1[cursor + 1] {
            cursor += 2;
            continue;
        }
        // That is okay, so check the byte before, then the byte after, and fail if one or the other is not the same.
        if (rle1[cursor] != rle1[cursor - 1]) && (rle1[cursor] != rle1[cursor + 3]) {
            cursor += 2;
            continue;
        }
        // If the byte before was the good one, move the cursor back one byte
        if rle1[cursor] == rle1[cursor - 1] {
            cursor -= 1;
        }
        // We are now at a run of 4
        {
            // Copy out the slice from the start cursor until the end of the run
            out.extend_from_slice(&rle1[start..cursor + 4]);
            // Create a vec of the repeating byte with a length taken from the byte following the run,
            //  and add that data to the output
            out.extend(vec![rle1[cursor]; usize::from(rle1[cursor + 4])]);
            cursor += 5;
            start = cursor;
        }
        cursor += 1;
    }
    out.extend_from_slice(&rle1[start..rle1.len()]);
    out
}
