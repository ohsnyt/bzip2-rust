use log::{error, info, trace};

use crate::bitstream::bitwriter::BitWriter;

use super::huffman_code_from_weights::improve_code_len_from_weights;
use crate::compression::compress::Block;
use std::cmp::Ordering;
use std::io::Error;

#[derive(Eq, PartialEq, PartialOrd, Ord, Debug, Clone)]
pub enum NodeData {
    Kids(Box<Node>, Box<Node>),
    Leaf(u16),
}
#[derive(Eq, PartialEq, Ord, Debug, Clone)]
pub struct Node {
    pub weight: u32,
    pub depth: u8,
    pub syms: u32,
    pub node_data: NodeData,
}
impl Node {
    /// Create a new node
    pub fn new(weight: u32, depth: u8, syms: u32, node_data: NodeData) -> Node {
        Node {
            weight,
            depth,
            syms,
            node_data,
        }
    }
}
impl PartialOrd for Node {
    /// Sort Nodes by decreasing weight and decreasing symbol value
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if other.weight == self.weight {
            return Some(other.syms.cmp(&self.syms));
        }
        Some(other.weight.cmp(&self.weight))
    }
}

#[allow(clippy::unusual_byte_groupings)]
/// Encode MTF/RLE2 data using Julian's multi-table system.
/// We need a BitWriter, adequately spedified block data, and the number of iterations desired (Julian used 4).
/// Data is returned via the block.
pub fn huf_encode(bw: &mut BitWriter, block: &mut Block, iterations: usize) -> Result<(), Error> {
    // Get the length of this RLE2 compressed block
    let vec_end = block.rle2.len();
    // We can have 2-6 coding tables depending on how much data we have coming in.
    let table_count: usize = match vec_end {
        0..=199 => 2,
        200..=599 => 3,
        600..=1199 => 4,
        1200..=2399 => 5,
        _ => 6,
    };

    // Now we can initialize the coding tables based on our frequency counts
    let mut tables = init_tables(&block.freqs, table_count, block.eob);

    // And initialize a count of how many selectors we need, a vec to store them,
    let selector_count = (block.rle2.len() / 50) + 1;
    let mut selectors = vec![0_usize; selector_count];

    /*
     So now we have our tables divided out by frequency ratios. Each symbol in each table
     is either a 0 or a 15. At the end of this next loop, those will be adjusted as we test
     against real data. These adjusted numbers are used to build a huffman tree, and
     thereby the huffman codes.

     We will iterate four times (default value, can be changed in options) to improve the tables.
     ds: Nov 2022: Looking at trial runs, it seems that three trial runs gains almost the same
     value as four times. Each larger size gains slightly more in ascii texts. Therefor I added
     an option to let the user change the number of trial runs -- mostly for more testing purposes.
    */

    for iter in 0..iterations {
        // initialize favorites[] to 0 for each table/group, for reporting only
        let mut favorites = [0; 6];

        // Initilalize the total cost for this iteration (used only in reporting)
        let mut total_cost = 0;

        // Initialize "recalculated" frequency array for each table/group (for adjusting the tables)
        let mut rfreq = [[0u32; 258]; 6];

        // Reset the selectors on each iteration - not needed because we record them only on the last iteration
        //selectors.clear();

        /*
        Time to move through the input 50 bytes at a time. For each group of 50, we
        compute the best table to use based on the one that has the lowest "weight" cost.

        NOTE: Julian did a trick with rolling all six 16 bit arrays into 3 32 bit arrays.
        I'm not doing that here. If we do in the future, I believe we could use 1 128 bit array
        for the same purpose (or possibly 2 64 bit arrays, or a struct).

        Our goal is to find the coding table which has the lowest cost for this chunk
        of data, and record that in the selector table.
        */

        block.rle2.chunks(50).enumerate().for_each(|(i, chunk)| {
            // the cost array helps us find which table is best for each 50 byte chunk
            let mut cost = [0; 6];

            // Find the best table for the next chunk
            // For each symbol, iterate through each table and calculate that tables cost for the symbol
            chunk.iter().for_each(|symbol| {
                (0..table_count).for_each(|t| cost[t] += tables[t][*symbol as usize])
            });

            // Get the position of the lowest non-zero cost (or first if several have the same low cost)
            let bt = cost
                .iter()
                .position(|n| n == cost[0..table_count as usize].iter().min().unwrap())
                .unwrap() as usize;

            // Add that lowest cost to total_cost for the entire input data set
            total_cost += cost[bt];

            // For reporting only, increment the appropriate fave array with the index
            // so we know how many times this table was chosen as "best"
            favorites[bt] += 1;

            // Now that we know the best table, go get the frequency counts for
            // the symbols in this group of 50 bytes and store the freq counts into rfreq.
            // As we go through an iteration, this becomes cumulative.
            // This is used to improve the tables for the next iteration.
            chunk
                .iter()
                .for_each(|&symbol| rfreq[bt as usize][symbol as usize] += 1);

            // On the last iteration, collect the selector list
            if iter == iterations - 1 {
                selectors[i] = bt;
            } // End of the for_each loop, we've gone through the entire input (again).
        });

        info!(
            " pass {}: best cost is {}, grp uses are {:?}",
            iter + 1,
            total_cost / 8,
            favorites
        );

        if iter == iterations - 1 {
            trace!("Final tables:",);
            for (i, table) in tables.iter().enumerate().take(table_count) {
                trace!(
                    "\n      {}: {:?}",
                    i,
                    table
                        .iter()
                        .take(block.eob as usize + 1)
                        .collect::<Vec<_>>()
                );
            }
        }

        // Next we will call improve_code_len_from_weights on each of the tables we made.
        // This makes actual node trees based off our weighting. This will put the
        // improved weights into the weight arrays. As mentioned, we do this 4 times.
        // for t in 0..table_count as usize {
        //     improve_code_len_from_weights(&mut tables[t], &rfreq[t], block.eob);
        // }
        (0..table_count).for_each(|t| {
            improve_code_len_from_weights(&mut tables[t], &rfreq[t], block.eob);
            trace!(
                "\nIter {}: Tbl {}:\n{:?}",
                iter,
                t,
                tables[t]
                    .iter()
                    .take(block.eob as usize + 1)
                    .collect::<Vec<_>>()
            );
        });
    }
    /*
      All iterations are now done, and we have good tables and selectors.
      Time to make actual binary codes for reach table. Since we have good lengths,
      we can use the code_from_length function to quickly generate codes.
    */

    // Write out the the symbol maps, 16 bit L1 + 0-16 words of 16 bit L2 maps.
    trace!("\r\x1b[43mSymbol maps written at {}.     \x1b[0m", bw.loc());
    for word in &block.sym_map {
        bw.out16(*word);
        trace!("\r\x1b[43m{:0>16b}     \x1b[0m", word);
    }

    // Symbol maps are followed by a 3 bit number of Huffman trees that exist
    trace!("\r\x1b[43mTable count written at {}.     \x1b[0m", bw.loc());
    bw.out24((3 << 24) | table_count as u32);

    // Then a 15 bit number indicating the how many selectors are used
    // (how many 50 byte groups are in this block of data)
    trace!(
        "\r\x1b[43mSelector count written at {}.     \x1b[0m",
        bw.loc()
    );
    bw.out24((15 << 24) | selector_count as u32);

    /*
    Selectors tell us which table is to be used for each 50 symbol chunk of input
    data in this block.

    Given a list of selectors such as [0,2,0,2,1,0], we can see that bytes
    1-50 are decoded by table 0, 51-100 are decoded by table 2, etc.

    HOWEVER, the selectors are written after a Move-To-Front transform, to save space.
    */

    // Initialize an index to the selector tables in preparation for the MTF transform for the selectors
    let mut table_idx = vec![0, 1, 2, 3, 4, 5];

    // Prepare the output selector vec
    let mut mtf_selectors = vec![0_usize; selector_count];

    // ...then do the MTF transform of the selector vector
    for i in 0..selector_count {
        // Get the index of the next selector
        let mut idx = table_idx.iter().position(|c| c == &selectors[i]).unwrap();
        // Write it to the MTF version of the selector vector
        mtf_selectors[i as usize] = idx;
        // Adjust the  table index
        // For speed, don't take time to adjust the index if we don't need to.
        match idx {
            0 => {}
            1 => {
                table_idx.swap(0, 1);
            }
            _ => {
                // Shift each index at the front of selector "forward" one. Do blocks of 3 for speed.
                let temp_sel = table_idx[idx as usize];

                while idx > 2 {
                    table_idx[idx as usize] = table_idx[idx as usize - 1];
                    table_idx[idx as usize - 1] = table_idx[idx as usize - 2];
                    table_idx[idx as usize - 2] = table_idx[idx as usize - 3];
                    idx -= 3;
                }
                // ...then clean up any odd ones
                while idx > 0 {
                    table_idx[idx as usize] = table_idx[idx as usize - 1];
                    idx -= 1;
                }
                // ...and finally put the "new" symbol index at the front of the index.
                table_idx[0] = temp_sel;
            }
        }
    }

    // Now write out all the mtf'ed selectors
    trace!(
        "\r\x1b[43m{} Selectors written at {}.     \x1b[0m",
        mtf_selectors.len(),
        bw.loc()
    );
    for selector in &mtf_selectors {
        match selector {
            0 => bw.out24(0x01000000),
            1 => bw.out24(0x02000002),
            2 => bw.out24(0x03000006),
            3 => bw.out24(0x0400000e),
            4 => bw.out24(0x0500001e),
            5 => bw.out24(0x0600003e),
            _ => error!("Bad selector value of {}", selector),
        };
    }
    // All done with mtf_selectors.
    drop(mtf_selectors);

    /*
    Now create the huffman codes. We need to convert our weights to huffman codes.
    (And later we will want to use the BitWriter with those codes also.)
    We will need both a vec of all output code tables, and a temporary place
    to build each output-style table.

    Remember, our tables are full 258 size arrays. We've done indexing and move-to-
    front transforms, so we are using only the bottom portion of that array.

    We will shift from an array format to a vec, which allows us to use Rust's
    optimized sorting functions.
    */

    // Create the vec for the output-style code tables
    let mut out_code_tables = vec![];

    // For as many tables as we have, we have quite few steps to do
    #[allow(clippy::needless_range_loop)]
    for i in 0..table_count as usize {
        // Because we use a fixed array of tables for speed, we must use an index to get only the ones we want
        let table = tables[i];
        // Calculate the size once
        let sym_size = block.eob as usize + 1;
        // Now create a output-style table
        let mut out_codes = vec![(0_u16, 0_u32); sym_size];
        // ... and create a vec of the symbols actually used
        // let mut len_sym: Vec<(u32, u16)> = vec![(0_u32, 0_u16); sym_size];
        // for (i, &t) in table.iter().enumerate().take(sym_size) {
        //     len_sym[i] = (t, i as u16);
        // }
        let mut len_sym: Vec<(u32, u16)> = table.iter().enumerate().take(sym_size).fold(
            Vec::with_capacity(sym_size),
            |mut vec, (i, t)| {
                vec.push((*t, i as u16));
                vec
            },
        );
        // ... and sort that vec ascending by length
        len_sym.sort_unstable();

        /*
        Get the minimum length in use so we can create the "next code".
        Next_code is a tuple of length from len_sym and a 32 bit code we build.

        Codes are sequential within each length range. For example, for a length
        of 3, the codes would be 000, 001, 010, 011, etc.
        */
        // Initialize next_code to the length of the first (smallest) length, and 0.
        let mut next_code: (u32, u32) = (len_sym[0].0, 0);

        // Create a vec where we can store the codes.
        let mut sym_codes = vec![(0_u16, 0_u32); sym_size];

        /*
        When the length changes, do a shift left for each increment and continue. So
        for example, if the length is now 5 and the last code had a length of 3 and
        was 010, we would now start with 01000, 01001, 01010, etc.

        We store a version for the BitWriter, a format I also use in my hashmap
        in the decompression side. This is in addition to the format we need below.

        The length is in the most significant 8 bits, the code in the least.
        For example if the length is 5 and the code is 11111, we'd see
            01234567_XXXXXX_0123456780123456
            00000101_000000_0000000000011111
        X indicates space we never use. Excuse the odd _ marking. It is why I use
        #[allow(clippy::unusual_byte_groupings)]
        */
        // For each symbol...
        for (i, (len, sym)) in len_sym.iter().enumerate() {
            if *len != next_code.0 {
                next_code.1 <<= len - next_code.0;
                next_code.0 = *len;
            }
            // ...save a version of the code in the BitWriter format
            out_codes[i] = (*sym, len << 24 | next_code.1);

            // ...and also save it for encoding below.
            sym_codes[i] = (*sym, next_code.1);

            // Increment the next_code.1 counter to generate the next code
            next_code.1 += 1;
        }

        /*
        Next we write out the symbol lengths that will be used in the decompression.
        They start with an "origin" length of five bits taken from the first symbol.

        Each symbol's length (INCLUDING THE FIRST SYMBOL) will be output as the delta
        (difference) from the last symbol. Each delta is exactly 2 bits long, a 11 or
        a 10. The end of the delta is indicated with a single zero bit.
        It seems odd to me that we write the first symbol, which will ALWAYS have a
        delta of zero.
        */

        // The len_sym vec now needs to be sorted by symbol, not length
        len_sym.sort_unstable_by(|a, b| a.1.cmp(&b.1));

        // We write the origin as a five bit int
        let mut origin = len_sym[0].0;
        trace!(
            "\r\x1b[43mWriting origin {} for huffman map {} at {}.   \x1b[0m",
            origin,
            i,
            bw.loc()
        );
        bw.out24((5 << 24) | origin as u32);

        // ... and iterate through the entire symbol list writing the deltas
        for entry in len_sym.iter() {
            trace!(
                "\r\x1b[43mIndex for {} is {} (Delta = {}), written at {}.     \x1b[0m",
                entry.1,
                entry.0,
                origin as i32 - entry.0 as i32,
                bw.loc(),
            );
            // get the next length
            let (l, _) = entry;
            // create the delta from the last length
            let mut delta = *l as i32 - origin as i32;
            // put this new length into origin for the next iteration of this loop
            origin = *l;
            // write out the length delta as ±1 repeatedly
            loop {
                match delta.cmp(&0) {
                    // if the delta is greater than 0, write 0x10
                    Ordering::Greater => {
                        bw.out24(0x02_000002);
                        // subtract one from the delta and loop again
                        delta -= 1;
                    }
                    // if the delta is less than 0, write 0x11
                    Ordering::Less => {
                        bw.out24(0x02_000003);
                        // add one t the delta and loop again
                        delta += 1;
                    }
                    // if there is no delta, break out of this loop
                    Ordering::Equal => {
                        break;
                    }
                }
            }
            // write a single 0 bit to indicate we are done with this symbol's length code
            bw.out24(0x01_000000);
        }
        // Sort the output codes by symbol before saving them in the output tables
        out_codes.sort_unstable();
        out_code_tables.push(out_codes);
    }

    /*
    Now encode and write the data.
    Each symbol in the input is basically an index to the code.
    We do this using the 50 byte table selectors, so we have to switch that up regularly.
    */

    // For debugging
    let mut index = 0;
    for (idx, chunk) in block.rle2.chunks(50).enumerate() {
        let table_idx = selectors[idx];
        chunk.iter().for_each(|symbol| {
            bw.out24(out_code_tables[table_idx][*symbol as usize].1);
            trace!(
                "\r\x1b[43mRLE2 {} ({}) written at {}.   \x1b[0m",
                index,
                *symbol,
                bw.loc()
            );
            index += 1;
        })
    }

    // All done
    Ok(())
}

#[allow(clippy::unusual_byte_groupings)]
/// Encode MTF data using Julian's multi-table system.
/// In addition to the options and BitWriter, we need frequency counts,
/// the bwt key, crc, the symbol map, and eob symbol (last symbol).
pub fn huf_encode_old(
    bw: &mut BitWriter,
    block: &mut Block,
    iterations: usize,
) -> Result<(), Error> {
    // Get the length of this RLE2 compressed block
    let vec_end = block.rle2.len();
    // We can have 2-6 coding tables depending on how much data we have coming in.
    let table_count = match vec_end {
        0..=199 => 2,
        200..=599 => 3,
        600..=1199 => 4,
        1200..=2399 => 5,
        _ => 6,
    };

    // Initialize the tables to weights of 15. Since Rust requires compile time array
    // sizing, let's just make 6 even though we might need less.
    let mut tables = [[15_u32; 260]; 6];

    // Then set the soft limits to divide the data out to the tables.
    let portion_limit: u32 = vec_end as u32 / table_count;
    /* How this works is a bit weird.
    We initially make tables based on the frequency of the symbols. For example, say we
    have enough data for six tables. Some symbols will have greater frequency than other
    symbols - and because of our MTF, symbols like RUNA and RUNB will be very frequent in
    many cases.

    We will build the tables based on symbol frequency. We assign a weight of zero to each
    possible symbol for those symbols that are in this  table, and a weight of 15 for any
    symbol that doesn't get into this table. If we have lots of RUNA symbols, it is very
    possible that over 1/6 of the frequency will be RUNA symbols. So this table would have
    a weight of 0 given to RUNA and a weight of 15 given to every other symbol. The next
    table gets as many symbols as needed to get to the next 1/6th of the frequency, with
    weights similarly apportioned.

    After making these initial tables, we run through the data 50 bytes at a time and see
    which table results in the lowest "cost" for those 50 bytes. We adjust costs/weights
    and repeat three more times. Julian must have found that this works better than just
    doing a straight-up huffman tree based on frequencies of the entire block.
    */

    // Update our coding tables. Note: Tables 3 and 5 are unique in that they get
    // just shy of the limit rather than just over the limit. If we did not do this,
    // we may not get enough symbols in the last tables.

    // First set our table index to the last table we need, and the portion sum to 0.
    let mut table_index = table_count as usize - 1;
    let mut portion = 0;
    // For each symbol add the frequency to portion and set the weight value for this
    // symbol in this table to 0. If the current portion meets the portion limit
    // (based on how many groups we have, and remembering the special limits for
    // tables 2 and 4) increment the table index to point to the next table and
    // reset the portion sum to 0. Keep going through all the symbols.
    for (i, freq) in block.freqs.iter().enumerate().take(block.eob as usize + 1) {
        let f = freq;
        if portion + f > portion_limit && (table_index == 2 || table_index == 4) {
            table_index = table_index.saturating_sub(1);
            tables[table_index][i] = 0;
            portion = *f;
            if portion > portion_limit {
                tables[table_index][i] = 0;
                table_index = table_index.saturating_sub(1);
                portion = 0;
            }
        } else {
            portion += f;
            tables[table_index][i] = 0;
            if portion > portion_limit {
                table_index = table_index.saturating_sub(1);
                portion = 0;
            }
        };
    }

    /*
     So now we have our tables divided out by frequency ratios. Each symbol in each table
     is either a 0 or a 15. At the end of this next loop, those will be adjusted as we test
     against real data. These adjusted numbers are used to build a huffman tree, and
     thereby the huffman codes.

     We will iterate four times (default value, can be changed in options) to improve the tables.
     ds: Nov 2022: Looking at trial runs, it seems that three trial runs gains almost the same
     value as four times. Each larger size gains slightly more in ascii texts. Therefor I added
     an option to let the user change the number of trial runs -- mostly for more testing purposes.
    */

    // Remember for later how many selectors we will have, and where we store them
    let mut selector_count = 0;
    let mut selectors = vec![];

    for iter in 0..iterations {
        // initialize favorites[] to 0 for each table/group
        let mut favorites = [0; 6];

        // initialize "recalculated" frequency array for each table/group
        let mut rfreq = [[0u32; 260]; 6];

        // Initialized counters for how many selectors we will have, a vec to store them,
        selector_count = 0;
        selectors.clear();

        // Initilalize the total cost for this iteration (used only in reporting)
        let mut total_cost = 0;

        /*
        Time to move through the input 50 bytes at a time. For each group of 50, we
        compute the best table to use based on the one that has the lowest "weight" cost.

        NOTE: Julian did a trick with rolling all six 16 bit arrays into 3 32 bit arrays.
        I'm not doing that here. If we do in the future, I believe we could use 1 128 bit array
        for the same purpose (or possibly multiple usize arrays).

        Our goal is to find the coding table which has the lowest cost for this chunk
        of data, and record that in the selector table.
        */

        // initialize chunk counters
        let mut start: usize = 0;
        let the_end = vec_end;

        while start < the_end {
            let end = (start + 50).min(the_end);

            // the cost array helps us find which table is best for each 50 byte chunk
            let mut cost = [0; 6];

            // Read through the next chunk of 50 symbols (of input) to find the best
            // table for these 50 symbols
            for &byte in block.rle2.iter().take(end as usize).skip(start) {
                // For each table...
                for t in 0..table_count as usize {
                    // increment the appropriate cost array with the weight of the symbol
                    cost[t] += tables[t][byte as usize];
                }
            }

            // Find the table with the lowest (best) "icost" (Julian's term)
            // Get the lowest non-zero value
            let min = cost[0..table_count as usize].iter().min().unwrap();
            // ...and get the position of it in the array
            let bt = cost.iter().position(|n| n == min).unwrap() as usize; // returns the FIRST lowest

            // Add that lowest cost to total_cost for the entire input data set
            total_cost += cost[bt];

            // Oncrement the appropriate fave array with the index to this table
            // so we know how many times this table was chosen as "best"
            favorites[bt] += 1;

            // Record the table index into the selector list
            selectors.push(bt);

            // increment the selector count
            selector_count += 1;

            // Now that we know the best table, go get the frequency counts for
            // the symbols in this group of 50 bytes and store the freq counts into rfreq.
            // as we go through an iteration, this becomes cumulative.
            for &symbol in block.rle2.iter().take(end as usize).skip(start) {
                rfreq[bt as usize][symbol as usize] += 1;
            }

            // Prepare to get the next group of 50 bytes from the input
            start = end;
        } // End of the while loop, we've gone through the entire input (again).
        trace!(
            "\n pass {}: size is {}, grp uses are {:?}",
            iter + 1,
            total_cost / 8,
            favorites
        );

        // Next we will call improve_code_len_from_weights on each of the tables we made.
        // This makes actual node trees based off our weighting. This will put the
        // improved weights into the weight arrays. As mentioned, we do this 4 times.
        for t in 0..table_count as usize {
            improve_code_len_from_weights(&mut tables[t], &rfreq[t], block.eob);
        }
    }
    /*
      4 iterations are now done, and we have good tables and selectors.
      Time to make actual binary codes for reach table. Since we have good lengths,
      we can use the code_from_length function to quickly generate codes.
    */

    // Next are the symbol maps, 16 bit L1 + 0-16 words of 16 bit L2 maps.
    for word in &block.sym_map {
        bw.out16(*word);
    }

    // Symbol maps are followed by a 3 bit number of Huffman trees that exist
    bw.out24((3 << 24) | table_count);

    // Then a 15 bit number indicating the how many selectors are used
    // (how many 50 byte groups are in this block of data)
    bw.out24((15 << 24) | selector_count);

    /*
    Selectors tell us which table is to be used for each 50 symbol chunk of input
    data in this block.

    Given a list of selectors such as [0,2,0,2,1,0], we can see that bytes
    1-50 are decoded by table 0, 51-100 are decoded by table 2, etc.

    HOWEVER, the selectors are written after a Move-To-Front transform, to save space.
    */

    // Initialize an index to the tables to do a MTF transform for the selectors
    let table_idx = vec![0, 1, 2, 3, 4, 5];

    // While we could do the MTF transform in place, we need the original table to do the encoding
    let mtf_selectors = selectors
        .iter()
        .fold((Vec::new(), table_idx), |(mut o, mut s), x| {
            let i = s.iter().position(|c| c == x).unwrap();
            let c = s.remove(i);
            s.insert(0, c);
            o.push(i);
            (o, s)
        })
        .0;

    // Now write out all the mtf'ed selectors
    for selector in &mtf_selectors {
        match selector {
            0 => bw.out24(0x01000000),
            1 => bw.out24(0x02000002),
            2 => bw.out24(0x03000006),
            3 => bw.out24(0x0400000e),
            4 => bw.out24(0x0500001e),
            5 => bw.out24(0x0600003e),
            _ => error!("Bad selector value of {}", selector),
        };
    }
    // All done with mtf_selectors.
    drop(mtf_selectors);
    /*
    Now create the huffman codes. We need to convert our weights to huffman codes.
    (And later we will want to use the BitWriter with those codes also.)
    We will need both a vec of all output code tables, and a temporary place
    to build each output-style table.

    Remember, our tables are full 258 size arrays. We've done indexing and move-to-
    front transforms, so we are using only the bottom portion of that array.

    We will shift from an array format to a vec, which allows us to use Rust's
    optimized sorting functions.
    */

    // Create the vec for the output-style code tables
    let mut out_code_tables = vec![];

    // For as many tables as we have, we have quite few steps to do
    #[allow(clippy::needless_range_loop)]
    for i in 0..table_count as usize {
        // Because we have a fixed array of tables, we must use an index to get only the ones we want
        let table = tables[i];
        // First create a output-style table
        let mut out_codes = vec![];
        // ... and create a vec of the symbols actually used
        let mut len_sym: Vec<(u32, u16)> = vec![];
        for (i, &t) in table.iter().enumerate().take(block.eob as usize + 1) {
            len_sym.push((t, i as u16));
        }
        // ... and sort that vec
        len_sym.sort_unstable();

        /*
        Get the minimum length in use so we can create the "next code".
        Next_code is a tuple of length from len_sym and a 32 bit code we build.

        Codes are sequential within each length range. For example, for a length
        of 3, the codes would be 000, 001, 010, 011, etc.
        */
        // Initialize next_code to the length of the first (smallest) length, and 0.
        let mut next_code: (u32, u32) = (len_sym[0].0, 0);

        // Create a vec where we can store the codes.
        let mut sym_codes = vec![];

        /*
        When the length changes, do a shift left for each increment and continue. So
        for example, if the length is now 5 and the last code had a length of 3 and
        was 010, we would now start with 01000, 01001, 01010, etc.

        We store a version for the BitWriter, a format I also use in my hashmap
        in the decompression side. This is in addition to the format we need below.

        The length is in the most significant 8 bits, the code in the least.
        For example if the length is 5 and the code is 11111, we'd see
            01234567_XXXXXX_0123456780123456
            00000101_000000_0000000000011111
        X indicates space we never use. Excuse the odd _ marking. It is why I use
        #[allow(clippy::unusual_byte_groupings)]
        */
        // For each symbol...
        for (len, sym) in &len_sym {
            if *len != next_code.0 {
                next_code.1 <<= len - next_code.0;
                next_code.0 = *len;
            }
            // ...save a version of the code in the BitWriter format
            out_codes.push((*sym, len << 24 | next_code.1));

            // ...and also save it for encoding below.
            sym_codes.push((*sym, next_code.1));

            // Increment the next_code.1 counter to generate the next code
            next_code.1 += 1;
        }

        /*
        Next we write out the symbol lengths that will be used in the decompression.
        They start with an "origin" length of five bits taken from the first symbol.

        Each symbol's length (INCLUDING THE FIRST SYMBOL) will be output as the delta
        (difference) from the last symbol. Each delta is exactly 2 bits long, a 11 or
        a 10. The end of the delta is indicated with a single zero bit.
        It seems odd to me that we write the first symbol, which will ALWAYS have a
        delta of zero.
        */

        // The len_sym vec now needs to be sorted by symbol, not length
        len_sym.sort_by(|a, b| a.1.cmp(&b.1));

        // We write the origin as a five bit int
        let mut origin = len_sym[0].0;
        bw.out24((5 << 24) | origin as u32);

        // ... and iterate through the entire symbol list writing the deltas
        for entry in len_sym.iter() {
            // get the next length
            let (l, _) = entry;
            // create the delta from the last length
            let mut delta = *l as i32 - origin as i32;
            // put this new length into origin for the next iteration of this loop
            origin = *l;
            // write out the length delta as ±1 repeatedly
            loop {
                match delta.cmp(&0) {
                    // if the delta is greater than 0, write 0x10
                    Ordering::Greater => {
                        bw.out24(0x02_000002);
                        // subtract one from the delta and loop again
                        delta -= 1;
                    }
                    // if the delta is less than 0, write 0x11
                    Ordering::Less => {
                        bw.out24(0x02_000003);
                        // add one t the delta and loop again
                        delta += 1;
                    }
                    // if there is no delta, break out of this loop
                    Ordering::Equal => {
                        break;
                    }
                }
            }
            // write a single 0 bit to indicate we are done with this symbol's length code
            bw.out24(0x01_000000);
        }
        out_codes.sort_unstable();
        out_code_tables.push(out_codes);
    }

    /*
    Now encode and write the data.
    Each symbol in the input is basically an index to the code.
    We do this using the 50 byte table selectors, so we have to switch that up regularly.
    */

    // Initialize a progress counter so we can keep track of the symbol count 0-49,
    // and a table index that we can change every 50 symbols as needed.
    let mut table_idx = 0;

    for (progress, symbol) in block.rle2.iter().enumerate() {
        // Switch the tables based on how many groups of 50 symbols we have done
        // Be sure to use the NON-MTF TABLES!
        if progress % 50 == 0 {
            table_idx = selectors[progress / 50];
        }
        bw.out24(out_code_tables[table_idx][*symbol as usize].1);
    }

    // All done
    Ok(())
}

/// Initialize 2-6 frequency tables based on the frequencies of the symbols in the data
fn init_tables(freqs: &[u32], table_count: usize, eob: u16) -> [[u32; 258]; 6] {
    // Initialize the tables to weights of 15. Since Rust requires compile time array
    // sizing, let's just make 6 even though we might need less.
    let mut tables = [[15_u32; 258]; 6];

    // Then set the soft limits to divide the data out to the tables.
    let mut portion_limit: u32 = freqs.iter().sum::<u32>() / table_count as u32;
    /* How this works is a bit weird.
    We initially make tables based on the frequency of the symbols. For example, say we
    have enough data for six tables. Some symbols will have greater frequency than other
    symbols - and because of our MTF, symbols like RUNA and RUNB will be very frequent in
    many cases.

    We will build the tables based on symbol frequency. We assign a weight of zero to each
    possible symbol for those symbols that are in this  table, and a weight of 15 for any
    symbol that doesn't get into this table. If we have lots of RUNA symbols, it is very
    possible that over 1/6 of the frequency will be RUNA symbols. So this table would have
    a weight of 0 given to RUNA and a weight of 15 given to every other symbol. The next
    table gets as many symbols as needed to get to the next 1/6th of the frequency, with
    weights similarly apportioned.

    After making these initial tables, we run through the data 50 bytes at a time and see
    which table results in the lowest "cost" for those 50 bytes. We adjust costs/weights
    and repeat three more times. Julian must have found that this works better than just
    doing a straight-up huffman tree based on frequencies of the entire block.
    */

    // Update our coding tables. Note: Tables 3 and 5 are unique in that they get
    // just shy of the limit rather than just over the limit. If we did not do this,
    // we may not get enough symbols in the last tables.

    // First set our table index to the last table we need, and the portion sum to 0.
    let mut table_index = table_count - 1;
    let mut portion = 0;
    // For each symbol add the frequency to portion and set the weight value for this
    // symbol in this table to 0. If the current portion meets the portion limit
    // (based on how many groups we have, and remembering the special limits for
    // tables 2 and 4) increment the table index to point to the next table and
    // reset the portion sum to 0. Keep going through all the symbols.
    for (i, freq) in freqs.iter().enumerate().take(eob as usize + 1) {
        let f = freq;
        if portion + f > portion_limit && (table_index == 2 || table_index == 4) {
            table_index = table_index.saturating_sub(1);
            tables[table_index][i] = 0;
            portion = *f;
            if portion > portion_limit {
                tables[table_index][i] = 0;
                table_index = table_index.saturating_sub(1);
                portion = 0;
            }
        } else {
            portion += f;
            tables[table_index][i] = 0;
            if portion > portion_limit {
                table_index = table_index.saturating_sub(1);
                portion = 0;
            }
        };
        // EXPERIMENTAL for case of huge frequency in one item
        // if f > &portion_limit {
        //     portion_limit = (freqs.iter().sum::<u32>() - *f) / (table_count - 1) as u32;
        // }
    }
    tables
}
