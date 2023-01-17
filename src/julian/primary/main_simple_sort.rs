use super::{main_gtu::main_gtu, main_sort::QsortData};

/// Simple sort for buckets of 20 or less, or depth greather than 14.
pub fn main_simple_sort(qs: &mut QsortData, lo: i32, hi: i32, d: i32, budget: &mut i32) {
    // It seems to me that we never use anything other than the first three values because of
    // the test "if i (lo + hp_incr > hi, break". After the break, we reduce the index by one.
    // Saved a few ms by making INCS a constant.
    const INCS: [i32; 14] = [
        1, 4, 13, 40, 121, 364, 1093, 3280, 9841, 29524, 88573, 265720, 797161, 2391484,
    ];

    // Return if nothing to sort
    let big_n = hi - lo + 1;
    if big_n < 2 {
        return;
    };

    // Initialize increment index. 
    let top_incr = INCS.into_iter().position(|i| i > big_n).unwrap_or_default();
    
    for incr_idx in (0..top_incr).rev() {
        let hp_incr = INCS[incr_idx as usize];
        let mut i = lo + hp_incr;
        loop {
            /*-- copy 1 --*/
            if i > hi {
                break;
            };
            let mut tmp_v = qs.bwt_ptr[i as usize];
            let mut j = i;

            while main_gtu(
                (qs.bwt_ptr[(j - hp_incr) as usize] as i32 + d) as usize,
                (tmp_v as i32 + d) as usize,
                qs,
                budget
            ) {
                qs.bwt_ptr[j as usize] = qs.bwt_ptr[(j - hp_incr) as usize];
                j -= hp_incr;
                if j <= (lo + hp_incr - 1) {
                    break;
                };
            }
            qs.bwt_ptr[j as usize] = tmp_v;
            i += 1;

            /*-- copy 2 --*/
            if i > hi {
                break;
            };
            tmp_v = qs.bwt_ptr[i as usize];
            j = i;
            while main_gtu(
                (qs.bwt_ptr[(j - hp_incr) as usize] as i32 + d) as usize,
                (tmp_v as i32 + d) as usize,
                qs,
                budget
            ) {
                qs.bwt_ptr[j as usize] = qs.bwt_ptr[(j - hp_incr) as usize];
                j -= hp_incr;
                if j <= (lo + hp_incr - 1) {
                    break;
                };
            }
            qs.bwt_ptr[j as usize] = tmp_v;
            i += 1;

            /*-- copy 3 --*/
            if i > hi {
                break;
            };
            tmp_v = qs.bwt_ptr[i as usize];
            j = i;
            while main_gtu(
                (qs.bwt_ptr[(j - hp_incr) as usize] as i32 + d) as usize,
                (tmp_v as i32 + d) as usize,
                qs,
                budget
            ) {
                qs.bwt_ptr[j as usize] = qs.bwt_ptr[(j - hp_incr) as usize];
                j -= hp_incr;
                if j <= (lo + hp_incr - 1) {
                    break;
                };
            }
            qs.bwt_ptr[j as usize] = tmp_v;
            i += 1;
            if *budget < 0 {
                return;
            };
        }
    }
}
