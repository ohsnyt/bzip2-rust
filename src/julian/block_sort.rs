use log::{debug, warn};

use crate::snyder::ss3::sais_entry;
//use crate::julian::fallback::fallback_sort::fallback_sort;

//use super::fallback::fallback_sort::fallback_sort;
//use super::fallback::fallback_sort_ds::fallback_sort_ds;
use super::primary::main_sort::{main_sort, QsortData};

/// Primary entry into Julian's BWT sorting system. This receives a ref to the block,  and the work factor.
/// It returns the key (usize) and data. NOT FULLY OPTIMIZED.
pub fn block_sort(block: &[u8]) -> (u32, Vec<u8>) {
    // Initialize variables for Julian's main sorting algorithm, cutting back vec sizes if not needed
    let mut qs = QsortData::new(block.len() as usize, 33_i32);
    let mut key: usize;
    let mut data: Vec<u8>;

    // If the size of the block us under 10k, use the fallbackSort function.
    if block.len() < 10000 {
        return sais_entry(block);
    } else {
        // budget_start is used to provide user statistics below
        let budget_start = (block.len() * 33) as i32;
        let mut budget = budget_start;

        let result = main_sort(block, &mut qs, &mut budget);
        if result.is_some() {
            debug!(
                "\nInitial budget: {}, Used: {}, Left: {}, block size: {}.",
                budget_start,
                budget_start - budget,
                budget,
                block.len(),
            );
            debug!(
                "\n{} work, {} block, {:.2} ratio.",
                budget_start - budget,
                block.len(),
                ((budget_start - budget) as f64 / block.len().max(1) as f64),
            );
            return result.unwrap();
        } else {
            warn!("    Too repetitive; using sais fallback sorting algorithm");
            return sais_entry(block);
        }
    }
}
