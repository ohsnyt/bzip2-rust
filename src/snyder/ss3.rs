// Simple SA-IS 3.0 David Snyder, using sentinels

#[allow(clippy::upper_case_acronyms)]
/// LMS struct holds commpressed L, S, and LMS values, plus counters used for validity checks.
struct LMS {
    /// Bit oriented vec of LMS type element indecies
    pub lms: Vec<u32>,
    /// Bit oriented vec of L and S type element indecies
    pub ls: Vec<u32>,
    // The following are primarily used in debugging - to simplify getting counts
    /// Position of the last element in the input data and LS/LMS vecs
    pub last: usize,
    /// Count of LMS type elements
    pub lms_count: usize,
    /// Count of L type elements
    pub l_count: usize,
    /// Count of S type elements
    pub s_count: usize,
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

        // Initialize ls and lms vecs
        self.lms = vec![0; data.len() / 32 + 1];
        self.ls = vec![0; data.len() / 32 + 1];
        // Initialize the final sentinel to S type as well as LMS type
        self.ls[self.last >> 5] |= 1_u32 << (self.last % 32);
        self.lms[self.last >> 5] |= 1_u32 << (self.last % 32);
        //self.debug();

        // Iterate backwards through the data determining L S and LMS elements. The sentinel at the end is an S, so
        // the last elmenet by definition must be an L type. We can start iterating left from here.
        //let mut idx = data.len() - 1;
        // S Type = 1, L type = 0;
        let mut current = 0;
        let mut prev = data[data.len() - 1];
        for (idx, &el) in data.iter().enumerate().take(data.len() - 1).rev() {
            match el.cmp(&prev) {
                std::cmp::Ordering::Less => {
                    self.ls[idx >> 5] |= 1_u32 << (idx % 32);
                    //self.debug();
                    current = 1;
                }
                std::cmp::Ordering::Equal => {
                    if current == 1 {
                        self.ls[idx >> 5] |= 1_u32 << (idx % 32);
                        //self.debug();
                    }
                }
                std::cmp::Ordering::Greater => {
                    if current == 1 {
                        // Mark previous element as lms
                        self.lms[(idx + 1) >> 5] |= 1_u32 << ((idx + 1) % 32);
                        current = 0;
                        // self.debug();
                    }
                }
            }
            prev = el;
        }
        // if current == 1 {
        //     // Mark previous element as lms
        //     self.lms[0 >> 5] |= 1_u32 << (0 % 32);
        //     //self.debug();
        // }
        // Before we leave, record the counts
        self.lms_count = self.lms.iter().map(|el| el.count_ones()).sum::<u32>() as usize;
        self.s_count = self.ls.iter().map(|el| el.count_ones()).sum::<u32>() as usize;
        self.l_count = data.len() - self.s_count;
    }

    /// Checks if element at index is an LMS element
    fn is_lms(&self, idx: usize) -> bool {
        //println!("Test: {:0>32b}", self.lms[idx >> 5]);
        if self.lms[idx >> 5] & (1_u32 << (idx % 32)) > 0 {
            return true;
        };
        false
    }

    // /// data element at idx is not an LMS element
    // pub fn is_not_lms(&self, idx: usize) -> bool {
    //     //println!("Test: {:0>32b}", self.lms[idx >> 5]);
    //     if self.lms[idx >> 5] & (1_u32 << (idx % 32)) == 0 {
    //         return true;
    //     };
    //     false
    // }

    /// data element at idx is an L element
    pub fn is_l(&self, idx: usize) -> bool {
        //println!("Test: {:0>32b}", self.ls[idx >> 5]);
        if self.ls[idx >> 5] & (1_u32 << (idx % 32)) == 0 {
            return true;
        };
        false
    }

    /// data element at idx is an S element
    fn is_s(&self, idx: usize) -> bool {
        //println!("Test: {:0>32b}", self.ls[idx >> 5]);
        if self.ls[idx >> 5] & (1_u32 << (idx % 32)) > 0 {
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

    fn debug(&self) {
        // Print count line to aid counting
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
//--- Done with LMS struct -------------------------------------------------------------------------------------

//-- Frequency Counts for Bucket Sorting -----------------------------------------------------------------------
/// Return frequency count of elements in the input vec. Size is the value of the largest element in the input.
fn bucket_sizes<T>(data: &[T], size: usize) -> Vec<u32>
where
    T: TryInto<usize> + Copy,
    T::Error: std::fmt::Debug,
{
    data.iter().fold(vec![0_u32; size], |mut freqs, &el| {
        freqs[el.try_into().unwrap_or_default()] += 1;
        freqs
    })
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
mod test {
    use super::*;
    #[test]
    pub fn freq_test() {
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
    T: TryInto<usize> + Copy + std::cmp::Ord + std::fmt::Display,
    T::Error: std::fmt::Debug,
{
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
pub fn entry(data: &[u8]) -> (u32, Vec<u8>) {
    // Replace the data with a "duval rotated" version, same data but split at a place determined by the duval
    // algorithm with parts a and b swapped. The Offset is the location of the original start of the data.
    let (data, offset) = rotate_duval(data);
    //DEBUG START
    // print!("     >");
    // for byte in &data {
    //     print!("{}", *byte as char);
    // }
    // println!("<");
    //DEBUG END

    let index = sa_is(&data, 256);
    let mut key = 0_u32;

    let mut bwt = vec![0_u8; data.len()];

    let duval_zero_position = (index.len() - offset) as u32;

    for i in 0..index.len() {
        if index[i] == duval_zero_position {
            key = i as u32
        }
        if index[i] == 0 {
            bwt[i] = data[data.len() - 1] as u8;
        } else {
            bwt[i] = data[index[i] as usize - 1];
        }
    }
    (key, bwt)
}

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
    return (
        names
            .iter()
            .filter(|el| el.is_some())
            .map(|el| el.unwrap())
            .collect::<Vec<u32>>(),
        offsets
            .iter()
            .filter(|el| el.is_some())
            .map(|el| el.unwrap())
            .collect::<Vec<u32>>(),
        current_name as usize + 1,
    );
}

fn make_summary_suffix_vec(summary_size: usize, lms: &LMS, mut summary: Vec<u32>) -> Vec<u32> {
    //Recurse if we had any identical LMS groups
    if summary_size != lms.lms_count {
        //println!("------------------------------------------------");
        //println!("-----------------RECURSION REQUIRED------------------");
        //println!("------------------------------------------------");
        summary = sa_is(&summary, summary_size);

        let mut summary_suffix_vec = vec![summary.len() as u32; summary.len() + 1];
        for i in 0..summary.len() {
            summary_suffix_vec[i + 1] = summary[i]
        }
        return summary_suffix_vec;
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
fn duval(input: &[u8]) -> usize {
    let mut final_start = 0;
    let n = input.len();
    let mut i = 0;

    while i < n {
        let mut j = i + 1;
        let mut k = i;
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

/// Compute lexicographically minimal rotation using the duval algorithm.
/// Returns the rotation and the offset.
fn rotate_duval(input: &[u8]) -> (Vec<u8>, usize) {
    let offset = duval(input);
    let mut buf = vec![];
    let (head, tail) = input.split_at(offset);
    buf.append(&mut tail.to_vec());
    buf.append(&mut head.to_vec());
    (buf, offset)
}

