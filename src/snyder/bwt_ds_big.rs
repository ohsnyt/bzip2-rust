use core::cmp::Ordering;
use log::error;
use rayon::prelude::*;
use std::mem;

use crate::compression::compress::Block;

/// Struct for Burrows-Wheeler-Transform data.
/// Contains the index to the original data order, a multi-byte sort
/// value (for speed). Sequential only (no threads).
#[derive(Clone, Eq, Debug)]
pub struct BwtKey {
    sort: usize,
    index: u32,
    depth: u16,
    symbol: u8,
}
/// Creator, requires an index number (u32), a sort value (usize), and a symbol value (u8).
impl BwtKey {
    pub fn new(index: u32, sort: usize, symbol: u8) -> Self {
        Self {
            sort,
            index,
            depth: 0,
            symbol,
        }
    }
}

/// Sort based on sort value only.
impl PartialOrd for BwtKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.sort.cmp(&other.sort))
    }
}
/// Sort based on sort value only.
impl Ord for BwtKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort.cmp(&other.sort)
    }
}
/// Equality tests both depth and sort values.
impl PartialEq for BwtKey {
    fn eq(&self, other: &Self) -> bool {
        (self.sort == other.sort) && (self.depth == other.depth)
    }
}

/// Sequential bwt sorting algorithm using Vec<usize> chunks instead of u8. ds, 2022.
/// ENTRY POINT
pub fn bwt_encode_big(block: &mut Block) {
    // Create usize sorting values
    let udata: Vec<usize> = udata_par_map(&block.data);
    // Create vec of custom structs for sorting
    let mut bwt_data = convert_to_bwt_data(&block.data, &udata);
    // Do initial sort of the data (***attempting unstable - check data carefully***)
    bwt_data.sort_unstable();

    // Repeatedly sort the data as long as we find identical sequences in it.
    let mut sub_depth = 1;
    while subsorting(&mut bwt_data, sub_depth, &udata) {
        sub_depth += 1;
    }
    // Return key and sorted data via block
    if let Some(key) = bwt_data
        .iter()
        .enumerate()
        .find(|(_, el)| el.index == 0)
        .map(|(i, _)| i as u32)
    {
        block.key = key;
        bwt_data
            .par_iter()
            .map(|el| el.symbol)
            .collect_into_vec(&mut block.data);
    }
}

/// Parallel update BwtKey data after sort
fn subsorting(data: &mut [BwtKey], rundepth: u32, udata: &Vec<usize>) -> bool {
    // Create tuples of all identical sort key sequences
    let mut seqs: Vec<(usize, usize)> = Vec::new();
    // Limit local variables to this block
    {
        // Local variable looks for runs. Done sequentially because we don't want to
        // accidentally split data in the middle of a run.
        let mut run = 1_usize;
        for i in 1..data.len() {
            if data[i - 1] == data[i] {
                run += 1;
            } else {
                if run > 1 {
                    seqs.push((i - run, i));
                }
                run = 1;
            }
        }

        //Exit with false if we didn't find any runs
        if seqs.is_empty() {
            return false;
        }
    }

    // Otherwise update the keys and sort the sequences
    seqs.iter().for_each(|(start, end)| {
        update_bwt_keys(&mut data[*start..*end], rundepth, udata);
        data[*start..*end].sort_unstable();
    });

    // Return true (we sorted something)
    true
}

/// Convert data to BwtKey vector
fn convert_to_bwt_data(data: &[u8], udata: &[usize]) -> Vec<BwtKey> {
    let end = data.len();
    data.iter()
        .enumerate()
        .map(|(i, _)| {
            BwtKey::new(
                ((end - i) % end) as u32,
                udata[(end - i) % end],
                data[(end - 1) - i % end],
            )
        })
        .collect::<Vec<BwtKey>>()
}

/// Update bwt_keys to next depth level
fn update_bwt_keys(mut data: &mut [BwtKey], depth: u32, udata: &Vec<usize>) {
    let end = udata.len();
        let offset = (std::mem::size_of::<usize>()) * (depth as usize);
        for i in 0..data.len() {
            data[i].sort = udata[(data[i].index as usize + offset) % end];
            data[i].depth = depth as u16;
        }
}

/// Create fast usize sorting data from input.
/// Combines multiple input u8s into one usize.
/// Currently designed for 64, 32, 16 and 8 bit system architectures.
pub fn udata_par_map(data: &[u8]) -> Vec<usize> {
    // Get the OS memory size
    let size = mem::size_of::<usize>();

    // Pad out the data by usize-1 more data by repeating the input as needed
    let mut ext_data = data.to_vec();
    let mut extend = size - 1;
    let newsize = data.len() + extend;
    while ext_data.len() < newsize {
        ext_data.extend_from_slice(&data[0..(extend).min(data.len())]);
        extend -= (extend).min(data.len());
    }

    // Match on usize length (in bytes)
    match size {
        /* When Rust goes to a 128 bit usize, add this code
        16 => ext_data
        .windows(size)
        .map(|w| {
            (w[0] as usize) << 120
            | (w[1] as usize) << 112
            | (w[2] as usize) << 104
            | (w[3] as usize) << 96
            | (w[4] as usize) << 88
            | (w[5] as usize) << 80
            | (w[6] as usize) << 72
            | (w[7] as usize) << 64
            | (w[8] as usize) << 56
            | (w[9] as usize) << 48
            | (w[10] as usize) << 40
            | (w[11] as usize) << 32
            | (w[12] as usize) << 24
            | (w[13] as usize) << 16
            | (w[14] as usize) << 8
            | (w[15] as usize)
        })
        .collect(),
        */
        8 => ext_data
            .windows(size)
            .enumerate()
            .map(|(_, w)| {
                (w[0] as usize) << 56
                    | (w[1] as usize) << 48
                    | (w[2] as usize) << 40
                    | (w[3] as usize) << 32
                    | (w[4] as usize) << 24
                    | (w[5] as usize) << 16
                    | (w[6] as usize) << 8
                    | (w[7] as usize)
            })
            .collect(),
        4 => ext_data
            .windows(size)
            .enumerate()
            .map(|(_, w)| {
                (w[0] as usize) << 24
                    | (w[1] as usize) << 16
                    | (w[2] as usize) << 8
                    | (w[3] as usize)
            })
            .collect(),
        2 => ext_data
            .windows(size)
            .enumerate()
            .map(|(_, w)| (w[0] as usize) << 8 | (w[1] as usize))
            .collect(),
        1 => ext_data.iter().map(|b| *b as usize).collect(),
        _ =>
        // Unplanned OS architecture - possibly 8 bit system.
        {
            error!("Unplanned OS architecture. Aborting.");
            panic!()
        }
    }
}
