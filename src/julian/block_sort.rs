use log::{trace, warn};

use crate::compression::compress::Block;

use super::fallback::fallback_sort::fallback_sort;
use super::primary::main_sort::{main_sort, QsortData};

/// Primary entry into Julian's BWT sorting system. This receives a ref to the block,  and the work factor.
/// It returns the key (usize) and data.
pub fn block_sort(block: &mut Block, qs: &mut QsortData) {
    // If the size of the block us under 10k, use the fallbackSort function.
    if block.end < 10000 {
        fallback_sort(block)
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
        block.budget = block.end as i32 * ((block.budget as i32 - 1) / 3);
        let budget_init = block.budget;

        main_sort(block, qs);

        trace!(
            "\nWork depleated: {}, block size: {}.",
            budget_init - block.budget,
            block.end,
        );
        if block.budget < 0 {
            warn!("    too repetitive; using fallback sorting algorithm");
            fallback_sort(block);
        }
    };
}
