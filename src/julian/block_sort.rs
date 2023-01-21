use log::{debug, warn};

use crate::compression::compress::Block;
use crate::julian::fallback::fallback_sort::fallback_sort;

//use super::fallback::fallback_sort::fallback_sort;
use super::fallback::fallback_sort_ds::fallback_sort_ds;
use super::primary::main_sort::{main_sort, QsortData};

/// Primary entry into Julian's BWT sorting system. This receives a ref to the block,  and the work factor.
/// It returns the key (usize) and data. NOT FULLY OPTIMIZED.
pub fn block_sort(block: &mut Block) {
    // Initialize the struct for Julian's main sorting algorithm, cutting back vec sizes if not needed
    let mut qs = QsortData::new(block.end as usize, block.budget);

    // If the size of the block us under 10k, use the fallbackSort function.
    if block.end < 100000 {
        fallback_sort_ds(block)
    } else {
        /* Julian note:
           (block.budget-1) / 3 puts the default-factor-30
           transition point at very roughly the same place as
           with v0.1 and v0.9.0.
           Not that it particularly matters any more, since the
           resulting compressed stream is now the same regardless
           of whether or not we use the main sort or fallback sort.
        */
        if block.budget < 1 {
            block.budget = 1
        };
        if block.budget > 100 {
            block.budget = 100
        };

        // budget_init(ial) is used to provide user statistics below
        let budget_init = block.end as i32 * ((block.budget as i32 - 1) / 3);
        let mut budget = budget_init;

        main_sort(block, &mut qs, &mut budget);

        if budget < 0 {
            warn!("    Too repetitive; using fallback sorting algorithm");
            fallback_sort_ds(block);
        } else {
            debug!(
                "\nInitial budget: {}, Used: {}, Left: {}, block size: {}.",
                budget_init,
                budget_init - budget,
                budget,
                block.end,
            );
        }
    };
}
