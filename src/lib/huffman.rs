use log::{debug, error, info, trace};

use super::bitwriter::BitWriter;
use super::huffman_code_from_weights::improve_code_len_from_freqs;
use std::cmp::Ordering;
use std::io::Error;

#[derive(Eq, PartialEq, Debug)]
pub enum NodeData {
    Kids(Box<Node>, Box<Node>),
    Leaf(u16),
}

#[derive(Eq, PartialEq, Debug)]
pub struct Node {
    pub frequency: u32,
    pub depth: u8,
    pub node_data: NodeData,
}
impl Node {
    pub fn new(frequency: u32, depth: u8, node_data: NodeData) -> Node {
        Node {
            frequency,
            depth,
            node_data,
        }
    }
}

#[allow(clippy::unusual_byte_groupings)]
/// Encode MTF data using Julian's multi-table system.
/// In addition to the options and BitWriter, we need frequency counts,
/// the bwt key, crc, the symbol map, and eob symbol (last symbol).
pub fn huf_encode(
    bw: &mut BitWriter,
    input: &[u16],
    freq_out: &[u32; 258],
    symbol_map: Vec<u16>,
    eob: u16,
) -> Result<(), Error> {
    // We can have 2-6 coding tables depending on how much data we have coming in.
    let table_count = match input.len() {
        0..=199 => 2,
        200..=599 => 3,
        600..=1199 => 4,
        1200..=2399 => 5,
        _ => 6,
    };
    debug!("We need {} huffman code tables", table_count);

    // Initialize the tables to weights of 15. Since Rust requires compile time array
    // sizing, let's just make 6 even though we might need less.
    let mut tables = [[15_u32; 258]; 6];

    // Then set the soft limits to divide the data out to the tables.
    let portion_limit: u32 = input.len() as u32 / table_count;
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

    // First set our table index to table 0, and the portion sum to 0.
    let mut table_index = 0;
    let mut portion = 0;
    // For each symbol add the frequency to portion and set the weight value for this
    // symbol in this table to 0. If the current portion meets the portion limit
    // (based on how many groups we have, and remembering the special limits for
    // tables 3 and 5) increment the table index to point to the next table and
    // reset the portion sum to 0. Keep going through all the symbols.
    for (i, freq) in freq_out.iter().enumerate().take(eob as usize + 1) {
        let f = freq;
        if portion + f > portion_limit && (table_index == 3 || table_index == 5) {
            tables[table_index][i] = 0;
            table_index += 1;
            portion = 0;
        } else {
            portion += f;
            tables[table_index][i] = 0;
            if portion > portion_limit {
                table_index += 1;
                portion = 0;
            }
        };
    }
    debug!("Created {} tables based on frequency ratios.", table_count);

    /*
     So now we have our tables divided out by frequency ratios. Each symbol in each table
     is either a 0 or a 15. At the end of this next loop, those will be adjusted as we test
     against real data. These adjusted numbers are used to build a huffman tree, and
     thereby the huffman codes.

     We will iterate four times to improve the tables. Each time we will try to make codes
     of 17 bits or less. If we can't, we will cut the weights down and try that iteration
     again.
    */

    // Remember for later how many selectors we will have, and where we store them
    let mut selector_count = 0;
    let mut selectors = vec![];

    for iter in 0..4 {
        debug!("Starting iteration {}", iter);
        // initialize favorites[] to 0 for each table/group
        let mut favorites = [0; 6];

        // initialize "recalculated" frequency array for each table/group
        let mut rfreq = [[0u32; 258]; 6];

        // Initialized counters for how many selectors we will have, a vec to store them,
        selector_count = 0;
        selectors = vec![];

        // Initilalize the total cost for this iteration (used only in reporting)
        let mut total_cost = 0;

        /*
        Time to move through the input 50 bytes at a time. For each group of 50, we
        compute the best table to use based on the one that has the lowest "weight" cost.

        NOTE: Julian did a trick with rolling all six 16 bit arrays into 3 32 bit arrays.
        I'm not doing that here. If we do in the future, we could use 1 128 bit array
        for the same purpose, I beleive.

        Our goal is to find the coding table which has the lowest cost for this chunk
        of data, and record that in the selector table.
        */

        // initialize chunk counters
        let mut start: usize = 0;
        let the_end = input.len();
        // the cost array helps us find which table is best for each 50 byte chunk
        let mut cost = [0; 6];

        while start <= the_end {
            let end = (start + 49).min(the_end);

            // Read through the next chunk of 50 symbols (of input) to find the best
            // table for these 50 symbols
            for &byte in input.iter().take(end as usize).skip(start) {
                // For each table...
                for t in 0..table_count as usize {
                    // increment the appropriate cost array with the weight of the symbol
                    cost[t] += tables[t][byte as usize];
                }
            }

            // Find the table with the lowest (best) "icost" (Julian's term)
            // bt = best table
            let bt = *cost.iter().min().unwrap() as usize; // returns the FIRST lowest

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
            // as we go through the input file, this becomes cumulative for each "best" table.
            for &symbol in input.iter().take(end as usize).skip(start) {
                rfreq[bt as usize][symbol as usize] += 1;
            }
            // Prepare to get the next group of 50 bytes from the input
            start = end + 1;
        } // End of the while loop, we've gone through the entire input (again).
        info!(
            " pass {}: size is {}, grp uses are {:?}",
            iter + 1,
            total_cost / 8,
            favorites
        );

        // Next we will call improve_code_len_from_freqs on each of the tables we made.
        // This makes actual node trees based off our weighting. This will put the
        // improved weights into the weight arrays. As mentioned, we do this 4 times.
        for t in 0..table_count as usize {
            trace!("Initial codes {:?}", tables[t]);
            improve_code_len_from_freqs(&mut tables[t], &rfreq[t], eob);
            trace!("Improved codes {:?}", tables[t]);
        }
    }
    /*
      4 iterations are now done, and we have good tables and selectors.
      Time to make actual binary codes for reach table. Since we have good lengths,
      we can use the code_from_length function to quickly generate codes.
    */

    debug!("Writing symbol_map starting at {}", bw.loc());
    // Next are the symbol maps , 16 bit L1 + 0-16 words of 16 bit L2 maps.
    for word in symbol_map {
        bw.out16(word);
    }

    // Symbol maps are followed by a 3 bit number of Huffman trees that exist
    debug!(
        "Writing table_count ({}) starting at {}",
        table_count,
        bw.loc()
    );
    bw.out24((3 << 24) | table_count);

    debug!(
        "Writing selector_count ({}) starting at {}",
        selector_count,
        bw.loc()
    );
    // Then a 15 bit number indicating the how many selectors are used
    // (how many 50 byte groups are in this block of data)
    bw.out24((15 << 24) | selector_count);

    debug!(
        "Writing {} selectors starting at {}",
        selectors.len(),
        bw.loc()
    );
    debug!("Selectors are {:?}", selectors);

    /*
    Selectors tell us which table is to be used for each 50 symbol chunk of input
    data in this block.

    Given a list of selectors such as [0,2,0,2,1,0], we can see that bytes
    1-50 are decoded by table 0, 51-100 are decoded by table 2, etc.

    HOWEVER, the selectors are written after a Move-To-Front transform, to save space.
    */
    // Initialize an index to the tables
    let mut table_idx = vec![0, 1, 2, 3, 4, 5];

    // Create a move-to-front vec for the selectors
    let mut mtf_selectors = vec![];
    // ...and do the mtf transform on the selector list
    for selector in selectors.iter_mut() {
        let mut v = *selector as usize;
        let tmp = table_idx[v];
        while v > 0 {
            table_idx[v] = table_idx[v - 1];
            v -= 1;
        }
        table_idx[0] = tmp;
        mtf_selectors.push(tmp);
    }

    // Now write out all the mtf'ed selectors
    for selector in &mtf_selectors {
        match selector {
            0 => bw.out24(0x01000000),
            1 => bw.out24(0x02000002),
            2 => bw.out24(0x03000004),
            3 => bw.out24(0x0400000e),
            4 => bw.out24(0x0500001e),
            5 => bw.out24(0x0600003e),
            _ => error!("Bad selector value of {}", selector),
        };
    }

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
    for i in 0..table_count as usize {
        // First create a output-style table
        let mut out_codes = vec![];
        // Then grab the matching weight table
        let table = tables[i];
        // ... and create a vec of the symbols actually used
        let mut len_sym: Vec<(u32, u16)> = vec![];
        for (i, &t) in table.iter().enumerate().take(eob as usize + 1) {
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
        debug!(
            "Writing a table with an origin of {} starting at {}",
            origin,
            bw.loc()
        );
        bw.out24((5 << 24) | origin as u32);
        trace!("Length symbol table is {:?}", len_sym);

        // ... and iterate through the entire symbol list writing the deltas
        for entry in len_sym.iter() {
            // get the next length
            let (l, _) = entry;
            // create the delta from the last length
            let mut delta = *l as i32 - origin as i32;
            // put this new length into origin for the next iteration of this loop
            origin = *l;
            // write out the length delta as ±1 repeatedly
            debug!("Writing length {} with delta of {}", l, delta);
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
    trace!("tables are {:?}", out_code_tables);

    /*
    Now encode and write the data.
    Each symbol in the input is basically an index to the code.
    We do this using the 50 byte table selectors, so we have to switch that up regularly.
    */

    // Initialize a progress counter so we can keep track of the symbol count 0-49,
    // and a table index that we can change every 50 symbols as needed.
    let mut progress = 0;
    let mut table_idx = 0;

    for (progress, symbol) in input.into_iter().enumerate() {
        // Switch the tables based on how many groups of 50 symbols we have done
        if progress % 50 == 0 {
            table_idx = mtf_selectors[table_idx / 50];
            debug!(
                "Chunk {}, table {}, output file location {}",
                progress / 50,
                table_idx,
                bw.loc()
            );
        }
        trace!(
            "symbol {}, table {}, code: {:032b}",
            symbol,
            table_idx,
            out_code_tables[table_idx][*symbol as usize].1
        );
        bw.out24(out_code_tables[table_idx][*symbol as usize].1);
    }
    debug!("Done at {}", bw.loc());

    // All done
    Ok(())
}

#[test]
fn huf_encode_decode_simple() {
    //let input = "Goofy test".as_bytes();
    //assert_eq!(huf_decode(&huf_encode(input).unwrap()), input)
}
