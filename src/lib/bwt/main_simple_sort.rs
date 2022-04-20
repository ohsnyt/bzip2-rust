//use log::{debug, error};

use super::main_gtu::main_gtu;

pub fn main_simple_sort(
    bwt_ptr: &mut [u32],
    block_data: &[u16],
    quadrant: &mut Vec<u16>,
    end: usize,
    lo: i32,
    hi: i32,
    d: i32,
    budget: &mut i32,
) {
    let incs = vec![
        1, 4, 13, 40, 121, 364, 1093, 3280, 9841, 29524, 88573, 265720, 797161, 2391484,
    ];

    let big_n = hi - lo + 1;
    if big_n < 2 {
        return;
    };

    let mut hp:i32 = 0;
    while incs[hp as usize] < big_n {
        hp += 1;
    }
    hp -= 1;

    while hp >= 0 {
        let h = incs[hp as usize];
        let mut i = lo + h;
        loop {
            /*-- copy 1 --*/
            if i > hi {
                break;
            };
            let mut v = bwt_ptr[i as usize];
            let mut j = i;
            // if i == 121 {
            //     println!("Pause here")
            // }
            while main_gtu(
                bwt_ptr[(j - h) as usize] as i32 + d,
                v as i32 + d,
                block_data,
                quadrant,
                end,
                budget,
            ) {
                bwt_ptr[j as usize] = bwt_ptr[(j - h) as usize];
                j -= h;
                if j <= (lo + h - 1) {
                    break;
                };
            }
            bwt_ptr[j as usize] = v;
            i += 1;

            /*-- copy 2 --*/
            if i > hi {
                break;
            };
            v = bwt_ptr[i as usize];
            j = i;
            while main_gtu(
                bwt_ptr[(j - h) as usize] as i32 + d,
                v as i32 + d,
                block_data,
                quadrant,
                end,
                budget,
            ) {
                bwt_ptr[j as usize] = bwt_ptr[(j - h) as usize];
                j -= h;
                if j <= (lo + h - 1) {
                    break;
                };
            }
            bwt_ptr[j as usize] = v;
            i += 1;

            /*-- copy 3 --*/
            if i > hi {
                break;
            };
            v = bwt_ptr[i as usize];
            j = i;
            while main_gtu(
                bwt_ptr[(j - h) as usize] as i32 + d,
                v as i32 + d,
                block_data,
                quadrant,
                end,
                budget,
            ) {
                bwt_ptr[j as usize] = bwt_ptr[(j - h) as usize];
                j -= h;
                if j <= (lo + h - 1) {
                    break;
                };
            }
            bwt_ptr[j as usize] = v;
            i += 1;
            if *budget < 0 {
                return;
            };
        }
        hp -= 1;
    }
}
