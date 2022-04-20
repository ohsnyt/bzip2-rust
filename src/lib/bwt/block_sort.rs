use log::{debug, info};

use super::{fallback_sort::fallback_sort, main_sort::main_sort};

pub fn block_sort(mut block_data: &[u8], mut work_factor: u32) -> (usize, Vec<u8>) {
    let end = block_data.len();
    let mut bwt_data;
    let mut key;

    //const OVERSHOOT: usize = 34;
    // If the size of the block us under 10k, use the fallbackSort function.
    if end < 10000 {
        (key, bwt_data) = fallback_sort(block_data);
    } else {
        /* (work_factor-1) / 3 puts the default-factor-30
           transition point at very roughly the same place as
           with v0.1 and v0.9.0.
           Not that it particularly matters any more, since the
           resulting compressed stream is now the same regardless
           of whether or not we use the main sort or fallback sort.
        */
        if work_factor < 1 {
            work_factor = 1
        };
        if work_factor > 100 {
            work_factor = 100
        };
        let budget_init: i32 = end as i32 * ((work_factor as i32 - 1) / 3);
        let mut budget = budget_init;

        (budget, key, bwt_data) = main_sort(block_data, budget);
        info!(
            " {} work, {} block, ratio {}",
            budget_init - budget,
            end,
            (budget_init - budget) / (end as i32).min(1)
        );
        if budget < 0 {
            debug!("    too repetitive; using fallback sorting algorithm");
            (key, bwt_data) = fallback_sort(&mut block_data);
        }
    }

    return (key, bwt_data.to_vec());
}
