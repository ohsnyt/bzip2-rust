/*
Stream orient Run Length Encoder. Being stream oriented allows this encoder to work with
the logic that divides a block so that the BWT encoding can received as close to the
maximum block size as possible without overrunning the 19 byte buffer.

When we find a run of 4 or more, we have to return the run counter as well as the non-matching 
byte that marked the end of the run. Hence the .next() function will return a tuple of option 
u8s, not just one u8.

Since it is important that a run of four always close with a count u8 after it, it is
important that the calling function loop with logic similar to this to flush out the
counter if we are in the middle of a run, and reset the encoder before restarting again:
    // Initialize encoder
    let mut rle = Encode::new();
    // Set block size. A size less than 4 makes no sense.
    let max = 15;
    // Initialize a vec to receive the RLE data
    let mut block: Vec<u8> = Vec::with_capacity(max + 19);

// Loop through the block
    for el in contents {
        if let Some(byte) = rle.next(el) {
            block.push(byte)
        }
        // Check if we are done with a block, but not in the middle of run
        if block.len() >= max && !rle.run {
            if let (Some(byte), part_b) = rle.next(*el) {
                block.push(byte);
                if part_b.is_some() {
                    block.push(part_b.unwrap())
                }
        }            }
            //... do stuff here with the RLE encoded full block ...
        }
    }
    // Flush the encoder at the end in case we 
    if let Some(byte) = rle.flush() {
        block.push(byte)
    }
    //... do stuff here with the RLE encoded partial block ...

 */
/// Encoder for RLE 1 (any run of 4+ identical bytes), prior to calculating block size & BWT. Returns tuple
#[derive(Clone, Copy)]
pub struct Encode {
    prev: Option<u8>,
    run_count: u32,
    pub run: bool,
}
impl Default for Encode {
    fn default() -> Self {
        Self::new()
    }
}
impl Encode {
    /// Create a new, empty encoder
    pub fn new() -> Self {
        Self {
            prev: None,
            run_count: 0,
            run: false,
        }
    }
    /// Look for identical runs of 4+ bytes. Return Some(byte), or None if we are
    /// in the middle of a run and just counting.
    pub fn next(&mut self, byte: u8) -> (Option<u8>, Option<u8>) {
        // If we have seen two identical bytes
        if Some(byte) == self.prev {
            match self.run_count {
                // First time we have a match, set the counter to 2 and output the byte
                0 => {
                    self.run_count = 2;
                    (Some(byte), None)
                }
                // Third byte, increment the counter again and output the byte
                2 => {
                    self.run_count += 1;
                    (Some(byte), None)
                }
                // Fourth identical byte. Mark the start of a run
                // Increment counter and output byte
                3 => {
                    self.run = true;
                    self.run_count += 1;
                    (Some(byte), None)
                }
                // We cannot exceed 260 bytes in a run, so terminate the run by
                // outputing the counter and resetting the encoder.
                260 => {
                    self.run_count = 0;
                    self.run = false;
                    self.prev = None;
                    (Some(255_u8), Some(byte))
                }
                // Anything between 3 and 260, just increment the counter and return None
                _ => {
                    self.run_count += 1;
                    (None, None)
                }
            }
        // If we found a non-matching byte, check if we just finished a run
        } else if self.run {
            // If so, reset the run indicator, remember this new byte for the next loop,
            // prepare to output the counter, reset the counter and output the counter.
            self.run = false;
            self.prev = Some(byte);
            let temp = self.run_count as u8 - 4;
            self.run_count = 0;
            (Some(temp), Some(byte))
        } else {
            // If not, remember the current byte so we can compare with the next byte
            self.prev = Some(byte);
            // Reset the run counter 
            self.run_count = 0;
            // Output the byte
            (Some(byte), None)
        }
    }
    /// Make sure the current length of a run is output if the input ends in a run
    pub fn flush(&mut self) -> Option<u8> {
        // Clear the comparison byte
        self.prev = None;
        // If we NOT are in the middle of a run
        if !self.run {
            // Return nothing
            None
        } else {
            // Otherwise return the counter and reset things for the next possible block
            let temp = self.run_count as u8 - 4;
            self.run_count = 0;
            self.run = false;
            Some(temp)
        }
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
    // Initialize our start counter and jump_past_search counter
    let mut start: usize = 0;
    let mut jump_past_search = 0;
    // Create a vec with the same capacity as the input
    let mut out = Vec::with_capacity(v.len());
    // loop until we get within 4 bytes of the end
    for i in 0..v.len() - 4 {
        // If we found a sequence of 4 identical bytes, we need to get past it
        if jump_past_search > 0 {
            // decrement the jump counter by one
            jump_past_search -= 1;
            // If not, look for a sequence of 4
        } else if v[i] == v[i + 1] && v[i] == v[i + 2] && v[i] == v[i + 3] {
            // If we found a sequence of 4, first write out everything from start to now.
            out.extend_from_slice(&v[start..i + 4]);
            // Create a vec of the identical characters, sizing the vec on the counter byte
            // after sequence of 4 we found
            let tmp = vec![v[i]; v[i + 4].into()];
            // ...and write that out
            out.extend(tmp);
            // Set the jump past variable to get us past the 4 plus count byte
            jump_past_search = 5;
            // and reset the start to be at the point just past the jump_past counter.
            start = i + jump_past_search;
        }
    }
    // Don't forget to write out any stuff we have skipped since the last start counter
    out.extend_from_slice(&v[start..v.len()]);
    // Return the transformed data
    out
}


#[test]
fn rle1_de_simple() {
    let input: Vec<u8> = vec![
        71, 111, 111, 102, 121, 32, 116, 101, 101, 101, 101, 4, 115, 116,
    ];
    assert_eq!(rle1_decode(&input), "Goofy teeeeeeeest".as_bytes());
}
