use log::error;

pub fn main_gtu(
    mut i1: i32,
    mut i2: i32,
    block_data: &[u16],
    quadrant: &[u16],
    end: usize,
    budget: &mut i32,
) -> bool {
    //debug
    // if i1 == 47 {
    //     println!("Pause here, main_gtu")
    // }
    if i1 == i2 {
        error!("mainGtU error")
    }

    if i1 > end as i32 - 1 - 12 + 34 {
        error!("main_gtu 1: i1 out of bounds will occur")
    }
    if i2 > end as i32 - 1 - 12 + 34 {
        error!("main_gtu 1: i2 out of bounds will occur")
    }
    
    /* 1 */
    let mut c1 = block_data[i1 as usize];
    let mut c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 2 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 3 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 4 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 5 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 6 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 7 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 8 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 9 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 10 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 11 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    /* 12 */
    c1 = block_data[i1 as usize];
    c2 = block_data[i2 as usize];
    if c1 != c2 {
        return c1 > c2;
    };
    i1 += 1;
    i2 += 1;

    if i1 > end as i32 - 1 - 12 + 34 {
        error!("main_gtu 1: i1 out of bounds will occur")
    }
    if i2 > end as i32 - 1 - 12 + 34 {
        error!("main_gtu 1: i2 out of bounds will occur")
    }
    let mut k: i32 = end as i32 + 8;
    while k >= 0 {
        /* 1 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };

        let mut s1 = quadrant[i1 as usize];
        let mut s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };
        i1 += 1;
        i2 += 1;

        /* 2 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };
        s1 = quadrant[i1 as usize];
        s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };
        i1 += 1;
        i2 += 1;

        /* 3 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };
        s1 = quadrant[i1 as usize];
        s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };
        i1 += 1;
        i2 += 1;

        /* 4 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };
        s1 = quadrant[i1 as usize];
        s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };
        i1 += 1;
        i2 += 1;

        /* 5 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };
        s1 = quadrant[i1 as usize];
        s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };
        i1 += 1;
        i2 += 1;

        /* 6 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };
        s1 = quadrant[i1 as usize];
        s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };
        i1 += 1;
        i2 += 1;

        /* 7 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };
        s1 = quadrant[i1 as usize];
        s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };
        i1 += 1;
        i2 += 1;

        /* 8 */
        c1 = block_data[i1 as usize];
        c2 = block_data[i2 as usize];
        if c1 != c2 {
            return c1 > c2;
        };
        s1 = quadrant[i1 as usize];
        s2 = quadrant[i2 as usize];
        if s1 != s2 {
            return s1 > s2;
        };

        i1 += 1;
        i2 += 1;
        if i1 >= end as i32 {
            i1 -= end as i32
        };
        if i2 >= end as i32 {
            i2 -= end as i32
        };
        k -= 8;
        *budget -= 1;

    }
    false
}
