// Simple SA-IS 3.0 David Snyder, using sentinels
const S: u32 = 1;
const LMS: u32 = 1;
const L: u32 = 0;

#[allow(clippy::upper_case_acronyms)]
/// LMS struct holds commpressed L, S, and LMS values, plus counters used for validity checks.
struct LMS {
    /// Bit oriented vec of LMS type element indecies
    lms: Vec<u32>,
    /// Bit oriented vec of L and S type element indecies
    ls: Vec<u32>,
    // The following are primarily used in debugging - to simplify getting counts
    /// Position of the last element in the input data and LS/LMS vecs
    last: usize,
    /// Count of LMS type elements
    lms_count: usize,
    /// Count of L type elements
    l_count: usize,
    /// Count of S type elements
    s_count: usize,
}

/// Create empty LMS struct
impl LMS {
    fn new() -> Self {
        Self {
            // Initialize ls and lms vecs
            ls: Vec::new(),
            lms: Vec::new(),
            // The following are primarily used in debugging - to simplify getting counts
            last: 0_usize,
            lms_count: 0_usize,
            l_count: 0_usize,
            s_count: 0_usize,
        }
    }

    /// Initialize an LMS struct based on data (which will be either u8 or u32).
    fn init<T>(&mut self, data: &[T])
    where
        T: TryInto<usize> + Copy + std::cmp::Ord,
        T::Error: std::fmt::Debug,
    {
        /*
        SA-IS builds a suffix tree, but BZIP2 wraps around the end of the string in the comparison.

        The (currently magic) solution is to use a Duval rotation of the data to prevent elements from
        miss-sorting.

        To store LMS info (s-type, l-type and lms-type elements) I will use a binary system where a set
        bit indicates an s-type and a a zero bit indicates a l-type in the ls vector. In the lms vector,
        a set bit indicates an lms element. Since 2^5 = 32, we can index to the correct u32 in the vecs by
        shifting the index right 5 bits. We can get the right element within that vec by using %32.

        This means that the temporary internal ls/lms info for 64 bytes can be stored in two u32 words (8 bytes).

        The use of sentinels also means that we must store ls/lms info at idx+1 (one position beyond the actual
        location in the data). We will be putting the sentinel into location 0.
        */

        // Initialize data end and search starting position
        self.last = data.len();

        // Initialize ls and lms vecs to all L (0 - no S or LMS found yet)
        self.lms = vec![L; data.len() / 32 + 1];
        self.ls = vec![L; data.len() / 32 + 1];
        // Initialize the final sentinel to S type as well as LMS type
        self.ls[self.last >> 5] |= S << (self.last % 32);
        self.lms[self.last >> 5] |= LMS << (self.last % 32);
        //self.debug();

        // Iterate backwards through the data determining L S and LMS elements. The sentinel
        // at the end is an S, so the last elmenet by definition must be an L type. We can
        // start iterating left from here.
        let mut current = L;
        let mut prev = data[data.len() - 1];
        for (idx, &el) in data.iter().enumerate().take(data.len() - 1).rev() {
            // Compare the current element with the previous element. If less or equal, this is an S
            match el.cmp(&prev) {
                std::cmp::Ordering::Less => {
                    self.ls[idx >> 5] |= S << (idx % 32);
                    //self.debug();
                    // Record that we are now working with a S type element
                    current = S;
                }
                // If the prev element is equal to this one, we need to compare to whether we are currently
                // working with S or L
                std::cmp::Ordering::Equal => {
                    if current == S {
                        // We are in a run of S, so we need to set this one to S. (L's are the unmarked varient)
                        self.ls[idx >> 5] |= S << (idx % 32);
                        //self.debug();
                    }
                }
                // If we found an L and we were in a run of S type elements, then the previous element must be an LMS
                std::cmp::Ordering::Greater => {
                    if current == S {
                        // Mark previous element as lms
                        self.lms[(idx + 1) >> 5] |= LMS << ((idx + 1) % 32);
                        current = L;
                        // self.debug();
                    }
                }
            }
            prev = el;
        }
        // Before we leave, take a couple nanoseconds to record the counts
        self.lms_count = self.lms.iter().map(|el| el.count_ones()).sum::<u32>() as usize;
        self.s_count = self.ls.iter().map(|el| el.count_ones()).sum::<u32>() as usize;
        self.l_count = data.len() - self.s_count;
    }

    /// Checks if element at index is set (is an LMS element)
    fn is_lms(&self, idx: usize) -> bool {
        if self.lms[idx >> 5] & (LMS << (idx % 32)) > 0 {
            return true;
        };
        false
    }

    /// data element at idx is not set (is an L)
    pub fn is_l(&self, idx: usize) -> bool {
        if self.ls[idx >> 5] & (S << (idx % 32)) == 0 {
            return true;
        };
        false
    }

    /// data element at idx is set (is an S)
    fn is_s(&self, idx: usize) -> bool {
        if self.ls[idx >> 5] & (S << (idx % 32)) > 0 {
            return true;
        };
        false
    }

    /// Test if LMS elements at data index a and b are NOT equal. Assumes a and be are lms elements.
    fn is_unequal_lms<T: std::cmp::PartialOrd + std::fmt::Display>(
        &self,
        data: &[T],
        a: usize,
        b: usize,
    ) -> bool {
        // If either is a sentinel, they are unequal
        if a == self.last || b == self.last {
            return true;
        }
        // Put smaller of a or b into first, and calculate difference between them
        let mut i = if a > b { b + 1 } else { a + 1 };
        let diff = if a > b { a - b } else { b - a };

        // Iterate through the data relative to both a and b checking for equality starting at the element past a
        while i != self.last - diff {
            let b = i + diff;
            // If both a and b are LMS elements, then we are past the segments and the segments were not unequal
            if self.is_lms(i) && self.is_lms(b) {
                return false;
            }

            // If only one was an LMS elements, then we are done and the segments were unequal
            if self.is_lms(i) || self.is_lms(b) {
                return true;
            }
            // If the next bytes are unequal, then the segments were unequal
            if data[i] != data[b] {
                return true;
            }
            i += 1;
        }
        // We worked through all the data, so the segments were not equal
        true
    }

    /// Test if LMS elements at data index a and b are NOT equal. Assumes a and be are lms elements.
    /// This is faster for larger blocks, but oddly slower for smaller blocks.
    fn is_unequal_lms_b<T: std::cmp::PartialOrd + std::fmt::Display>(
        &self,
        data: &[T],
        a: usize,
        b: usize,
    ) -> bool {
        // If either a or b is a sentinel, the elements are unequal
        if a == self.last || b == self.last {
            return true;
        }
        // Make sure we start at the element that is smaller
        let (c, diff) = if a > b {
            (b, (a - b).min(self.last - a))
        } else {
            (a, (b - a).min(self.last - b))
        };

        let mut length = 1;
        while !self.is_lms(c + length) && length < diff {
            length += 1;
        }

        if data[a..a + length] == data[b..b + length] {
            return false;
        }
        true
    }

    fn debug(&self) {
        // Print a "count line" to aid counting
        print!("    --");
        for i in 0..=self.last {
            print!("{}", i % 10);
        }
        println!("--  ");

        // Print LS data
        print!("  LS: ");
        for i in 0..=self.last {
            print!("{}", (self.ls[i >> 5] & 1_u32 << (i % 32) > 0) as u32);
        }
        println!("  ({} S & {} L elements)", self.s_count, self.l_count);

        // Print LMS data
        print!(" LMS: ");
        for i in 0..=self.last {
            print!("{}", (self.lms[i >> 5] & 1_u32 << (i % 32) > 0) as u32);
        }
        println!("  ({} LMS elements)", self.lms_count);
    }
}

#[cfg(test)]
mod test_lms {
    use super::*;
    #[test]
    pub fn lms_test() {
        let data = "caabage".as_bytes();
        let mut lms = LMS::new();
        lms.init(data);
        //cabbage - LSLLSLLS
        assert!(lms.is_l(0));
        assert!(lms.is_s(1));
        assert!(lms.is_s(2));
        assert!(lms.is_l(3));
        assert!(lms.is_s(4));
        assert!(lms.is_l(5));
        assert!(lms.is_l(6));
        assert!(lms.is_s(7));

        assert_eq!(lms.is_lms(0), false);
        assert_eq!(lms.is_lms(1), true);
        assert_eq!(lms.is_lms(2), false);
        assert_eq!(lms.is_lms(3), false);
        assert_eq!(lms.is_lms(4), true);
        assert_eq!(lms.is_lms(5), false);
        assert_eq!(lms.is_lms(6), false);
        assert_eq!(lms.is_lms(7), true);
    }
}
//--- Done with LMS struct ------------------------------------------------------------------------------------

use std::cmp::Ordering;

//-- Counts for Bucket Sorting --------------------------------------------------------------------------------
use rayon::prelude::*;

/// Return frequency count of elements in the input vec. Size is the value of the largest element in the input.
fn bucket_sizes<T>(data: &[T], size: usize) -> Vec<u32>
where
    T: TryInto<usize> + Copy,
    T::Error: std::fmt::Debug,
    T: Sync,
{
    // Use parallel method if more than 64k elements in the data
    if data.len() > 64_000 {
        // 16k is pretty much the sweet spot for chunk size.
        data.par_chunks(16_000)
            .fold(
                || vec![0_u32; size],
                |mut freqs, chunk| {
                    chunk
                        .iter()
                        .for_each(|&el| freqs[el.try_into().unwrap_or_default()] += 1);
                    freqs
                },
            )
            .reduce(
                || vec![0_u32; size],
                |s, f| s.iter().zip(&f).map(|(a, b)| a + b).collect::<Vec<u32>>(),
            )
    } else {
        let mut freqs = vec![0_u32; size];
        data.iter()
            .for_each(|&el| freqs[el.try_into().unwrap_or_default()] += 1);
        freqs
    }
}

/// Returns index to top positions of buckets for bucket sorting.
fn bucket_heads(buckets: &[u32]) -> Vec<u32> {
    buckets
        .iter()
        .enumerate()
        .fold(
            (vec![0_u32; buckets.len()], 1_u32),
            |(mut head, mut idx), (i, &count)| {
                head[i] = idx;
                idx += count;
                (head, idx)
            },
        )
        .0
}
/// Returns index to bottom positions of buckets for bucket sorting.
fn bucket_tails(buckets: &[u32]) -> Vec<u32> {
    buckets
        .iter()
        .enumerate()
        .fold(
            (vec![0_u32; buckets.len()], 1_u32),
            |(mut tail, mut idx), (i, &count)| {
                idx += count;
                tail[i] = idx - 1;
                (tail, idx)
            },
        )
        .0
}

#[cfg(test)]
mod test_bucket_prep {
    use super::*;
    #[test]
    pub fn freq_count_test() {
        let data = [2, 0, 1, 1, 0, 6, 4];
        let frq = bucket_sizes(&data, 7);
        assert_eq!(frq[0..7], vec![2, 2, 1, 0, 1, 0, 1]);
    }
    #[test]
    pub fn freq_head_test() {
        let data = [2, 0, 1, 1, 0, 6, 4];
        let freq = bucket_sizes(&data, 7);
        let heads = bucket_heads(&freq);
        assert_eq!(heads[0..7], vec![1, 3, 5, 6, 6, 7, 7]);
    }
    #[test]
    pub fn freq_tail_test() {
        let data = [2, 0, 1, 1, 0, 6, 4];
        let freq = bucket_sizes(&data, 7);
        let tails = bucket_tails(&freq);
        assert_eq!(tails[0..7], vec![2, 4, 5, 5, 6, 6, 7]);
    }
}
//-- End Frequency Counts for Bucket Sorting -------------------------------------------------------------------

//-- Bucket Sorting --------------------------------------------------------------------------------------------
/// Initial bucket sort of the LMS elements in the buckets vec.
fn initial_buckets_sort<T>(data: &[T], bkt_sizes: &[u32], lms: &LMS) -> Vec<Option<u32>>
where
    T: TryInto<usize> + Copy,
    T::Error: std::fmt::Debug,
{
    // Get the bucket tails info
    let mut tails = bucket_tails(bkt_sizes);

    // Initialize output vec to contain 1 more element that the input data
    let mut buckets = vec![None; data.len() + 1];
    buckets[0] = Some(lms.last as u32);

    // Find the LMS elements
    for idx in (0..lms.last).rev() {
        if lms.is_lms(idx) {
            buckets[tails[data[idx].try_into().unwrap_or_default()] as usize] = Some(idx as u32);
            tails[data[idx].try_into().unwrap_or_default()] -= 1;
            //println!("Buckets: {:?}", buckets);
        }
    }
    // Add the sentinel to the front
    buckets[0] = Some(data.len() as u32);
    buckets
}

/// Induce L type elements into the sort array after initial allocation of LMS elements
fn induced_sort_l<T>(data: &[T], buckets: &mut [Option<u32>], bkt_sizes: &[u32], lms: &LMS)
where
    T: TryInto<usize> + Copy,
    T::Error: std::fmt::Debug,
{
    // Get the bucket heads info
    let mut heads = bucket_heads(bkt_sizes);

    // Find L type elements that are left of the index and insert them into the buckets.
    // We can start at 0 and walk to the end with L type elements.
    for idx in 0..lms.last {
        // Only do buckets with a valid index
        if buckets[idx].is_some() {
            // Check if the element left of the element indexed by this bucket also an L type.
            let prev = if buckets[idx] == Some(0) {
                lms.last
            } else {
                buckets[idx].unwrap() as usize - 1
            };
            if lms.is_l(prev) {
                // If so, insert that l-type into the next free top spot in that bucket
                buckets[heads[data[prev].try_into().unwrap_or_default()] as usize] =
                    Some(prev as u32);
                // And adjust the location available for the next L-type in this bucket (if any)
                heads[data[prev].try_into().unwrap_or_default()] += 1;
                //DEBUG
                //lms.debug();
            }
        }
    }
}

/// Induce S type elements into the sort array after induced L sort
fn induced_sort_s<T>(data: &[T], buckets: &mut [Option<u32>], bkt_sizes: &[u32], lms: &LMS)
where
    T: TryInto<usize> + Copy + std::cmp::Ord,
    T::Error: std::fmt::Debug,
{
    // Get the bucket tails info
    let mut tails = bucket_tails(bkt_sizes);

    // Start at the right most known element and then iterate down.
    let mut idx = lms.last;
    // Prepare to loop back to here
    while idx > 0 {
        // Check if the element left of the element indexed by this bucket is an S type.
        if buckets[idx].is_none() {
            eprintln!("This should never happen. Idx is {}", idx);
            lms.debug(); // Pause here
        }
        // As long as we are not referencing the start of the data
        if buckets[idx] != Some(0) {
            // Get the previous element index
            let prev = buckets[idx].unwrap() as usize - 1;
            // If it is an S type
            if lms.is_s(prev) {
                // Insert/update that element into the next free bottom spot in the appropriate bucket
                let bkt_index = data[prev].try_into().unwrap_or_default();
                buckets[tails[bkt_index] as usize] = Some(prev as u32);
                // And adjust the location available for the next S type in this bucket (if any)
                tails[data[prev].try_into().unwrap_or_default()] -= 1;
                //DEBUG
                //lms.debug();
            }
        };
        idx -= 1;
    }
}

fn sa_is<T>(data: &[T], alphabet_size: usize) -> Vec<u32>
where
    T: TryInto<usize> + Copy + std::cmp::Ord + std::fmt::Display + std::marker::Sync,
    T::Error: std::fmt::Debug,
{
    // Don't attemp to process empty data.
    if data.is_empty() {
        return vec![];
    }

    // STEP 1: Build LMS info
    let mut lms = LMS::new();
    lms.init(data);
    //DEBUG
    //lms.debug();

    // STEP 2: Calculate buckets for bucket sorting
    let bkt_sizes = bucket_sizes(data, alphabet_size);

    // STEP 3: Do initial bucket sorting of LMS elements
    let mut buckets = initial_buckets_sort(data, &bkt_sizes, &lms);
    // Validity test for development
    if lms.lms_count != buckets.iter().filter(|&b| b.is_some()).count() {
        eprintln!(
            "Didn't initialize buckets properly. Missed {}.",
            lms.lms_count - buckets.iter().filter(|&b| b.is_some()).count()
        );
        debug_buckets('i', &buckets);
    }

    // STEP 4: Do induced L sort
    induced_sort_l(data, &mut buckets, &bkt_sizes, &lms);
    // Validity test for development
    if lms.s_count - lms.lms_count != buckets.iter().filter(|&b| b.is_none()).count() {
        println!(
            "Expected to have {} empty buckets after first induced_sort_l. Instead...",
            lms.s_count - lms.lms_count,
        );
        debug_nones(&buckets);
        debug_buckets('l', &buckets);
    }

    // STEP 5: Do induced S sort
    induced_sort_s(data, &mut buckets, &bkt_sizes, &lms);
    // Validity test for development
    if 0 != buckets.iter().filter(|&b| b.is_none()).count() {
        eprintln!(
            "Didn't complete s-induced sort properly. Missed {}.",
            buckets.iter().filter(|&b| b.is_none()).count()
        );
        debug_nones(&buckets);
        //debug_buckets('s', &buckets);
    }

    // STEP 6: Create Summary Suffix list from correctly sorted LMS elements
    // Create summary of unique lms elements.
    let (summary, offsets, summary_size) = make_summary(data, &mut buckets, &lms);
    // Create unique ordered elements, recursing if needed to ensure only unique lms elements exist.
    let summary_suffix_vec = make_summary_suffix_vec(summary_size, &lms, summary);

    //STEP 7: Do final bucket sort based on the summary. First clear the buckets
    (0..buckets.len()).for_each(|i| buckets[i] = None);

    // Get the bucket tails info
    let mut tails = bucket_tails(&bkt_sizes);

    // Place the LMS elements in the summary_suffix_vec
    for el in summary_suffix_vec.iter().skip(2).rev() {
        let data_index = offsets[*el as usize] as usize;
        let bucket_index = data[data_index].try_into().unwrap_or_default() as usize;
        buckets[tails[bucket_index] as usize] = Some(data_index as u32);
        tails[bucket_index] -= 1;
        // println!("Buckets: {:?}", buckets);
    }

    // Add the sentinel to the front
    buckets[0] = Some(data.len() as u32);

    // Validity test for development
    if lms.lms_count != buckets.iter().filter(|&b| b.is_some()).count() {
        eprintln!("Didn't initialize buckets for FINAL sort properly.");
        debug_buckets('F', &buckets);
    }

    // STEP 9: Do induced L sort
    induced_sort_l(data, &mut buckets, &bkt_sizes, &lms);
    // Validity test for development
    if lms.s_count - lms.lms_count != buckets.iter().filter(|&b| b.is_none()).count() {
        println!(
            "Expected to have {} empty buckets during first induced_sort_l. Instead...",
            lms.s_count - lms.lms_count,
        );
        debug_nones(&buckets);
        //debug_buckets('L', &buckets);
    }

    // STEP 10: Do induced S sort
    induced_sort_s(data, &mut buckets, &bkt_sizes, &lms);
    // Validity test for development
    if 0 != buckets.iter().filter(|&b| b.is_none()).count() {
        eprintln!(
            "Didn't complete s-induced sort properly. Missed {}.",
            buckets.iter().filter(|&b| b.is_none()).count()
        );
        debug_nones(&buckets);
        //debug_buckets('S', &buckets);
    }

    // Convert Option<u32> to u32 and return that vec
    buckets.iter().skip(1).map(|&el| el.unwrap()).collect()
}

/// Entry point for Simple SA-IS sort for Burrow-Wheeler Transform. Takes u8 slice and returns
/// u32 key and u8 vector in BWT format.
pub fn sais_entry(data: &[u8]) -> (u32, Vec<u8>) {
    /*
    SA-IS doesn't work in our context unless it is "duval rotated". We must have a lexicographically minimal
    rotation of the data or the conversion from the index to the BWT vec will be off.

    The duval rotation finds the "lexically minimal" point and splits and reorders the data around that point.

    Cudos to https://github.com/torfmaster/ribzip2, where I initially saw this concept in use.
    */

    // Do the rotation and return the reorganized data and offset to the original start of the data.
    let (data, offset) = rotate_duval(data);

    // Go do the sa-is sort, returning the index to the BWT.
    let index = sa_is(&data, 256);

    // Initialize the key. We find the actutal value in the loop below.
    let mut key = 0_u32;
    // Initialize the final vec
    let mut bwt = vec![0_u8; data.len()];
    // Get the offset to the original start of the data
    let duval_zero_position = (index.len() - offset) as u32;

    // Create the final BWT vec and find the actual key value
    for i in 0..index.len() {
        // Watch for the key location
        if index[i] == duval_zero_position {
            key = i as u32
        }
        // BWT is build from the data at the previous index location. Wrap around if the index is at 0.
        if index[i] == 0 {
            bwt[i] = data[data.len() - 1] as u8;
        } else {
            bwt[i] = data[index[i] as usize - 1];
        }
    }
    // Return the key and BWT data
    (key, bwt)
}

/// Create summary of LMS elements. Return vec of LMS names, vec of offsets and count of unique LMS names.
fn make_summary<T>(
    data: &[T],
    buckets: &mut [Option<u32>],
    lms: &LMS,
) -> (Vec<u32>, Vec<u32>, usize)
where
    T: TryInto<usize> + Copy + std::cmp::Ord + std::fmt::Display,
    T::Error: std::fmt::Debug,
{
    // Initialize temporary names vec
    let mut names: Vec<Option<u32>> = vec![None; buckets.len()];
    // Initialize temporary offset(pointer) list
    let mut offsets = vec![None; buckets.len()];
    // Initialize current name
    let mut current_name = 0_u32;
    // Initialize sentinel name
    names[buckets[0].unwrap() as usize] = Some(current_name);
    // Initialize sentinel pointer
    offsets[buckets[0].unwrap() as usize] = Some(lms.last as u32);
    // Initialize previous LMS to sentinel LMS value
    let mut prev_lms = buckets[0];

    // Iterate through the buckets looking for sequences of identical lms groups, skipping the sentinel
    for &ptr in buckets[1..].iter() {
        // Unwrap and convert to usize once - to make it easier to read
        if lms.is_lms(ptr.unwrap() as usize) {
            let curr_lms = ptr.unwrap() as usize;
            if lms.is_unequal_lms(data, prev_lms.unwrap() as usize, curr_lms) {
                prev_lms = Some(curr_lms as u32);
                current_name += 1;
            }
            // Store the results based on the pointer in the buckets
            names[curr_lms] = Some(current_name);
            offsets[curr_lms] = Some(curr_lms as u32);
        }
    }
    // Filter out non-lms elements and return names, offsets and count of unique LMS names
    (
        names.into_iter().flatten().collect::<Vec<u32>>(),
        offsets.into_iter().flatten().collect::<Vec<u32>>(),
        current_name as usize + 1,
    )
}

/// Create vec that contains only unique LMS elements, recursing if needed to ensure only unique LMS elements exist.
fn make_summary_suffix_vec(summary_size: usize, lms: &LMS, mut summary: Vec<u32>) -> Vec<u32> {
    // Recurse if we had any identical LMS groups when we made the summary (make_summary returned summary_size,
    //  the count of unique LMS elements)
    if summary_size != lms.lms_count {
        // Recurse
        summary = sa_is(&summary, summary_size);
        // The above recurses until there are no more duplicate LMS elements.
        // Now return the summary the recursion finally produced.
        let mut summary_suffix_vec = vec![summary.len() as u32; summary.len() + 1];
        summary_suffix_vec[1..(summary.len() + 1)].copy_from_slice(&summary[..]);
        summary_suffix_vec
    } else {
        // Make a summary suffix vec from summary with a new sentinel at the beginning.
        let mut summary_suffix_vec = vec![summary_size as u32; summary.len() + 1];
        // Fill in the rest of the vec from summary
        summary.iter().enumerate().for_each(|(idx, el)| {
            summary_suffix_vec[*el as usize + 1] = idx as u32;
        });
        summary_suffix_vec
    }
}

/// Debug function for the buckets.
fn debug_buckets(note: char, buckets: &[Option<u32>]) {
    println!(
        "{} Buckets: {:?}",
        note,
        (0..32.min(buckets.len())).fold(Vec::new(), |mut vec, i| {
            vec.push(buckets[i]);
            vec
        })
    );
}

/// Debug function to show which bucket elemements were missing.
fn debug_nones(buckets: &[Option<u32>]) {
    let mut indecies = (0..buckets.len() as u32).collect::<Vec<u32>>();
    buckets.iter().for_each(|b| {
        if b.is_some() {
            indecies.remove(b.unwrap() as usize);
        }
    });
    indecies.sort();
    eprintln!(" Didn't fill: {:?} ", indecies);
}

/// FROM https://github.com/torfmaster/ribzip2
fn duval_original(input: &[u8]) -> usize {
    let mut final_start = 0;
    let n = input.len();
    let mut i = 0;
    let mut j;
    let mut k;

    while i < n {
        j = i + 1;
        k = i;
        while j < n && input[k] <= input[j] {
            if input[k] < input[j] {
                k = i;
            } else {
                k += 1;
            }
            j += 1;
        }
        while i <= k {
            final_start = i;

            i += j - k;
        }
    }
    final_start
}

/// Compute the Lexicographically Minimal String Rotation
fn duval(input: &[u8]) -> usize {
    let n = input.len();
    if n < 2 {
        return 0;
    }
    let mut smallest = (input[0], 0);

    let mut this = 0;
    let mut next = 1;
    // Find the smallest run
    while this < n - 1 {
        if next >= n {
            next -= n
        };
        // If ever the next byte is smaller than the smallest, we have a new smallest run
        if input[next] < smallest.0 {
            smallest = (input[next], next);
            next += 1;
            continue;
        };
        // If the next byte is equal to this current byte, increment both and check the next
        if input[this] == input[next] {
            next += 1;
            this += 1;
            continue;
        }
        // If the next byte is smaller than this byte, we could have a new smallest run starting at next_start
        if input[next] < input[this] {
            // See if the next byte is different from the smallest
            if input[next] != smallest.0 {
                // If it is different, we can't have a new run. Go to the next byte
                next += 1;
                this += 1;
                continue;
            } else {
                // check which run is longer - the one starting at smallest or the one starting at next
                let (mut a, mut b) = (smallest.1, next);
                // Advance past the equal bytes
                while input[a] == input[b % n] {
                    a += 1;
                    b += 1;
                    if a == n {
                        break;
                    }
                }
                // If a is greater than b, run b is longer
                if input[a % n] > input[b % n] {
                    smallest = (input[next], next);
                }
                this = b;
                next = b + 1;
                continue;
            }
        }
        // If the next byte is larger than this one, start looking again at the next byte
        if input[next] > input[this] {
            next += 1;
            this += 1;
            continue;
        }
    }
    smallest.1
}


/// Compute lexicographically minimal rotation using the duval algorithm.
/// Returns the rotation and the offset.
fn rotate_duval(input: &[u8]) -> (Vec<u8>, usize) {
    // let offset = duval_original(input);
    let offset = duval(input);
    let mut buf = vec![];
    let (head, tail) = input.split_at(offset);
    buf.append(&mut tail.to_vec());
    buf.append(&mut head.to_vec());
    (buf, offset)
}

/// Given a sample slice of the data (5k suggested), compute the LMS complexity.
/// A complexity of less than 0.3 indicates that SA-IS is the better algorithm
/// than a multi-threaded version of the native block sort algorithm.
pub fn lms_complexity(data: &[u8]) -> f64 {
    // STEP 1: Build LMS info
    let mut lms = LMS::new();
    lms.init(&data);

    // STEP 2: Compute LMS complexity, ver 1.0
    lms.lms_count as f64 / data.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duval_test_a() {
        let data = "a".as_bytes();
        assert_eq!(duval(data), 0);
    }
    #[test]
    fn duval_test_ba() {
        let data = "ba".as_bytes();
        assert_eq!(duval(data), 1);
    }
    #[test]
    fn duval_test_aaaaaa() {
        let data = "aaaaaa".as_bytes();
        assert_eq!(duval(data), 0);
        //assert_eq!(duval_original(data), 0);
    }
    #[test]
    fn duval_test_aaaaab() {
        let data = "aaaaab".as_bytes();
        assert_eq!(duval(data), 0);
    }
    #[test]
    fn duval_test_aaaaba() {
        let data = "aaaaba".as_bytes();
        assert_eq!(duval(data), 5);
    }
    #[test]
    fn duval_test_aaabaa() {
        let data = "aaabaa".as_bytes();
        assert_eq!(duval(data), 4);
    }
    #[test]
    fn duval_test_aabaaa() {
        let data = "aabaaa".as_bytes();
        assert_eq!(duval(data), 3);
    }
    #[test]
    fn duval_test_abaaaa() {
        let data = "abaaaa".as_bytes();
        assert_eq!(duval(data), 2);
    }
    #[test]
    fn duval_test_baaaaa() {
        let data = "baaaaa".as_bytes();
        assert_eq!(duval(data), 1);
    }
    #[test]
    fn duval_test_baaaab() {
        let data = "baaaab".as_bytes();
        assert_eq!(duval(data), 1);
    }
    #[test]
    fn duval_test_abbbba() {
        let data = "abbbba".as_bytes();
        assert_eq!(duval(data), 5);
    }
    #[test]
    fn duval_test_baabaa() {
        let data = "baabaa".as_bytes();
        assert_eq!(duval(data), 1);
    }
    #[test]
    fn duval_test_abaabaaabaababaaabaaababaab() {
        let data = "abaabaaabaababaaabaaababaab".as_bytes();
        assert_eq!(duval(data), 14);
    }
}
