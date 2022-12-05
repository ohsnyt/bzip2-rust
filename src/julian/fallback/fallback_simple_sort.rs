// NOTE: Dropped this in favor of using the build-in Rust sort for vecs

/*---------------------------------------------*/
/*--- Fallback O(N log(N)^2) sorting        ---*/
/*--- algorithm, for repetitive blocks      ---*/
/*---------------------------------------------*/

/*---------------------------------------------*/

/* /// Sorts small bucket - could be slice between hi..lo if I reworked it.
pub fn fallback_simple_sort(freq_map: &mut [u32], block_data: &[u16], lo: i32, hi: i32, _h: i32) {
    //info!("f {:?}", &freq_map);

    if lo == hi {
        return;
    };
    if hi - lo > 3 {
        let mut i = hi - 4;
        while i >= lo {
            let tmp = freq_map[i as usize];
            let ec_tmp = block_data[tmp as usize];
            let mut j = i + 4;
            while j <= hi && ec_tmp > block_data[freq_map[j as usize] as usize] {
                freq_map[j as usize - 4] = freq_map[j as usize];
                j += 4;
            }
            freq_map[j as usize - 4] = tmp;
            i -= 1;
        }
    }
    let mut i = hi - 1;
    while i >= lo {
        let tmp = freq_map[i as usize];
        let ec_tmp = block_data[tmp as usize];
        let mut j = i + 1;
        while j <= hi && ec_tmp > block_data[freq_map[j as usize] as usize] {
            freq_map[j as usize - 1] = freq_map[j as usize];
            j += 1;
        }
        freq_map[j as usize - 1] = tmp;
        i -= 1;
    }
} 

/// Sorts slice using Rust's fast .sort_unstable_by
pub fn fallback_simple_sort2(map_slice: &mut [u32], block_data: &[u16]) {
    map_slice.sort_unstable_by(|a, b| block_data[*a as usize].cmp(&block_data[*b as usize]) );
}
*/