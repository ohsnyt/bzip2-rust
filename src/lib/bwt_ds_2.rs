use log::error;
use std::{
    cmp::{min, Ordering},
    iter::Enumerate,
};
use voracious_radix_sort::{RadixSort, Radixable};

const SYSBYTES: usize = (usize::BITS / 8) as usize;

/// Struct for main BTW data
#[derive(Copy, Clone, Debug)]
pub struct BWTData {
    /// SYSBYTES sized context of original data (8 bytes on 64-bit platform)
    key: usize,
    /// original sequence number
    ptr: usize,
    /// sorted (or not)
    sorted: bool,
    subgroup: u32,
}
impl PartialOrd for BWTData {
    fn partial_cmp(&self, other: &BWTData) -> Option<Ordering> {
        match self.key.partial_cmp(&other.key) {
            Some(std::cmp::Ordering::Greater) => Some(Ordering::Greater),
            Some(std::cmp::Ordering::Less) => Some(Ordering::Less),
            Some(std::cmp::Ordering::Equal) => self.subgroup.partial_cmp(&other.subgroup),
            None => self.subgroup.partial_cmp(&other.subgroup),
        }
    }
}
impl PartialEq for BWTData {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.subgroup == other.subgroup
    }
}

impl Radixable<usize> for BWTData {
    type Key = usize;
    #[inline]
    fn key(&self) -> Self::Key {
        self.key
    }
}

/// Struct for subsort indexing - secondary (tertiary, etc.) sorting
#[derive(Copy, Clone, Debug)]
pub struct SubsortData {
    /// Context for sorting substring
    key: usize,
    /// Pointer of original position within the substring
    ptr: usize,
    /// Copy of main block slice sorted in the subsort (to speed up "sorting" the main block)
    original: BWTData,
}
impl PartialOrd for SubsortData {
    fn partial_cmp(&self, other: &SubsortData) -> Option<Ordering> {
        self.key.partial_cmp(&other.key)
    }
}
impl PartialEq for SubsortData {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Radixable<usize> for SubsortData {
    type Key = usize;
    #[inline]
    fn key(&self) -> Self::Key {
        self.key
    }
}

/// Burrows-Wheeler-Transform - written by David Snyder.
/// This sorts the data with context to reduce resorting. It
/// returns a usize key and u8 vec of the sorted data.
pub fn bwt_encode(bytes: &[u8]) -> (usize, Vec<u8>) {
    // First get the size of the block.
    let end = bytes.len();

    // Add space for the context
    let mut b: Vec<u8> = bytes.to_vec();
    b.extend(bytes[0..8].iter());

    // Create a vec with context, pointer and sort status information.
    // (Iter below is faster than older fold version)
    let mut udata: Vec<BWTData> = b[0..end]
        .iter()
        .enumerate()
        .map(|(idx, _)| BWTData {
            key: usize::from_be_bytes(b[idx..(idx + SYSBYTES)].try_into().unwrap()),
            ptr: idx,
            sorted: false,
            subgroup: 0,
        })
        .collect::<Vec<BWTData>>();

    // We don't need b any more.
    drop(b);

    // Sort using the voracious multi-thread radix sort (2 threads works well)
    voracious_radix_sort::RadixSort::voracious_mt_sort(&mut udata, 2);

    // Mark items that are now fully sorted relative to their neighbors. First and last
    // elements in the vec are special cases.
    // First element.
    if udata[0].key != udata[1].key {
        udata[0].sorted = true;
    }
    // Middle elements.
    for i in 1..end - 1 {
        if udata[i - 1].key != udata[i].key && udata[i].key != udata[i + 1].key {
            udata[i].sorted = true;
        }
    }
    // Last element.
    if udata[end - 1].key != udata[end - 2].key {
        udata[end - 1].sorted = true;
    }

    // Now build an index to the pointers so we can find the original items fast.
    let mut index: Vec<usize> =
        udata
            .iter()
            .enumerate()
            .fold(vec![0_usize; udata.len()], |mut v, (idx, el)| {
                v[el.ptr] = idx;
                v
            });

    // We need to track how deep we are in sub-sorting so that we can calculate the offset
    // in terms of depth*SYSBYTES. We start with 1.
    let mut depth = 1;
    //println!("Depth is now {}", depth);
    // Create the a bucket for subsorting
    let mut bucket: (usize, usize);

    // Create a vec to hold subsort data - this is currently HUGE under the assumetion that
    // it is faster to waste space than calculate the true size needed.
    let mut subsort = vec![
        SubsortData {
            key: 0,
            ptr: 0,
            original: BWTData {
                key: 0,
                ptr: 0,
                sorted: false,
                subgroup: 0,
            },
        };
        end
    ];

    // While we still have data to sort.
    //   (This expression returns true only if any element is marked as not sorted)
    while udata.iter().any(|el| el.sorted == false) {
        let mut i = 0;
        let remaining_count = udata.iter().filter(|el| el.sorted == false).count();
        if remaining_count > 0 {
            log::info!(
                "---The subsort level {}: {} elements yet to sort.",
                depth,
                udata.iter().filter(|el| el.sorted == false).count()
            );
        }
        while i < end {
            if !udata[i].sorted {
                bucket = (i, get_bucket_length(&udata[i..]));
                sort_bucket(&mut index, &mut udata, bucket, depth, &mut subsort, end);
                i += bucket.1 - 1;
            }
            i += 1;
        }
        depth += 1;
    }
    log::info!("---Done sorting.",);

    // Get key and compute BWT output
    let mut key: usize = 0;
    let mut bwt: Vec<u8> = vec![0; end];
    for i in 0..end {
        if udata[i].ptr == 0 {
            key = i;
            bwt[i] = bytes[end - 1];
        } else {
            bwt[i] = bytes[udata[i].ptr - 1];
        }
    }
    (key, bwt)
}

//============== Helper functions =================
/// Sort sub-buckets of identical elements, marking udata elements if they were sorted.
fn sort_bucket(
    index: &mut [usize],
    udata: &mut [BWTData],
    (start, length): (usize, usize),
    depth: usize,
    subsort: &mut [SubsortData],
    end: usize,
) {
    // Added this error message for testing purposes. Should never happen.
    if length < 2 {
        error!("Oops - bucket too short");
        return;
    }

    // Populate the 'resuable' subsort vec we received
    for i in 0..length {
        subsort[i] = SubsortData {
            key: udata[index[(udata[start + i].ptr + depth * SYSBYTES) % end]].key,
            ptr: i,
            original: udata[start + i],
        };
    }

    // Quick solution when there are only two elements
    if length == 2 {
        match subsort[0].key.cmp(&subsort[1].key) {
            Ordering::Greater => {
                // Swap the two elements then the index pointers
                udata.swap(start, start + 1);
                // Swap the index pointers
                index.swap(udata[start].ptr, udata[start + 1].ptr);
                udata[start].sorted = true;
                udata[start + 1].sorted = true;
                return;
            }
            Ordering::Equal => return,
            Ordering::Less => {
                udata[start].sorted = true;
                udata[start + 1].sorted = true;
                return;
            }
        }
    }
    // Quick solution when all elements are the same
    if subsort[0..length].iter().all(|el| el.key == subsort[0].key) {
        return;
    }

    // Step 2: Sort the subsort elements, don't waste time with multi threaded sort
    //         on these smaller sorts.
    subsort[0..length].voracious_sort();

    // Step 3: Update the sort and subgroup info within the subsort
    let mut group_num = 1;
    let mut group_key = 0;
    //  Do the first element
    if subsort[0].key != (subsort[1].key) {
        subsort[0].original.sorted = true;
    } else {
        subsort[0].original.subgroup = group_num;
        group_key = subsort[0].key;
    }
    //  Then the middle elements
    for i in 1..length - 1 {
        if subsort[i - 1].key != subsort[i].key && subsort[i].key != subsort[i + 1].key {
            subsort[i].original.sorted = true;
        } else {
            if subsort[i].key != group_key {
                group_num += 1;
                group_key = subsort[i].key;
            }
            subsort[i].original.subgroup = group_num;
        }
    }
    //  And finally the last element
    if subsort[length - 1].key != subsort[length - 2].key {
        subsort[length - 1].original.sorted = true;
    } else {
        subsort[length - 1].original.subgroup = group_num;
    }

    // Step 3b: Re-Sort the subsort elements with the subgroup info
    subsort[0..length].voracious_sort();

    // Step 4: Replace the original data based on the subsort pointers.
    for i in 0..length {
        udata[start + i] = subsort[i].original;
        index[udata[start + i].ptr] = start + i;
    }
}

/// Returns the number of identical elements from the start of the slice.
fn get_bucket_length(slice: &[BWTData]) -> usize {
    slice
        .iter()
        .position(|&x| x != slice[0])
        .unwrap_or_else(|| slice.len())
}
