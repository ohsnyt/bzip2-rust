use super::huffman_code_from_weights::improve_code_len_from_freqs;
use super::report::report;
use super::{bitwriter::BitWriter, options::BzOpts};
use crate::lib::options::Verbosity::Chatty;
use std::cmp::Ordering;
use std::io::Error;
#[allow(clippy::unusual_byte_groupings)]
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
/// the bwt key, crc, and the symbol map.
pub fn huf_encode(
    opts: &mut BzOpts,
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

    // Initialize the tables to weights of 15. Since Rust requires compile time array
    // sizing, lets just make 6 even though we might need less.
    let mut tables = [[15_u32; 258]; 6];

    // Then set the soft limits to divide the data out to the tables.
    let portion_limit: u32 = input.len() as u32 / table_count;
    /* This is a bit weird, how it works. We initially make multiple tables based on the
    frequency of the symbols. For example if we have enough data for six tables, starting
    with the most frequently occuring symbols we add as many symbols to that table that we
    need to have that table have 1/6th of the frequency of the input data. We assign a
    weight of zero if the symbol is in the table, and a weight of 15 for any symbol that
    doesn't get into this table.  The next table gets as many symbols as needed to get
    to the next 1/6th of the frequency, with weights similarly apportioned.

    After making these initial tables, we run through the data 50 bytes at a time and see
    which table results in the lowest "cost". We adjust costs/weights and repeat three
    more times. Somehow it must work better than just doing a straight up tree.
    */

    // Update our coding tables. Note: Tables 3 and 5 are unique in that they get
    // just shy of the limit rather than just over the limit.
    let mut table_index = 0;
    let mut portion = 0;
    for (i, freq) in freq_out.iter().enumerate().take(eob as usize) {
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

    /*
     So now we have our length tables.

     We enter this next loop with each value holding a 0 or 15. At the end of this next
     loop, those will be adjusted as we test against real data. These adjusted numbers
     are used to build a huffman tree, and thereby the huffman codes.

     We will iterate four times to improve the tables. Each time we will try to make codes
     of 17 bits or less. If we can't, we will cut the weights down and try again.
    */

    // Remember for later how many selectors we will have, and where we store them
    let mut selector_count = 0;
    let mut selectors = vec![];

    for iter in 0..4 {
        // initialize fave[] to 0 for each table/group
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
        I'm not doing that here. Could we instead use 1 128 bit array for the same purpose?
        */

        // initialize chunk counters
        let mut start: usize = 0;
        let mut cost = [0; 6];

        println! {"Starting iteration {}", iter}

        /*
        Walk through the whole input data block in chunks of 50 bytes (or eob)
        adding the weighted "cost" of each symbol to the total cost for the table.
        Our goal is to find the coding table which has the lowest cost for this chunk
        of data, and record that in the selector table.
        */
        let the_end = input.len();
        while start <= the_end {
            let end = (start + 49).min(the_end);

            // println! {"Working on byte {} to {}. Total cost is {}", start, end, total_cost }
            // if end/50 >= 233 {
            //     println! {"   Start {}, end {}, the end {}.", start, end, the_end}
            // }

            // Read through a chunk of 50 bytes of data, updating the tables
            for &byte in input.iter().take(end as usize).skip(start) {
                let mtfv = byte;
                for t in 0..table_count as usize {
                    cost[t] += tables[t][mtfv as usize];
                }
            }

            // check each of the 2-6 groups to get the table with the lowest (best) icost
            // Set best cost (bc) to a very large cost initially
            let mut bc = 999999999;
            // and best table to table 0 for starters
            let mut bt = 0;
            // then find the table with the lowest (best) cost
            for (t, &item) in cost.iter().enumerate().take(table_count as usize) {
                if item < bc {
                    bc = item;
                    bt = t;
                }
            }

            // So now we have the table with the lowest icost. Add that cost to total_cost
            // for the entire input data set
            total_cost += bc;

            // increment the appropriate fave array with the index to this table
            // this lets us know how many times this table was chosen as "best"
            favorites[bt] += 1;

            // record the table index into the selector list
            selectors.push(bt);

            // increment the selector count
            selector_count += 1;

            // Now that we know the best table, go get the frequency counts for
            // the symbols in this group of 50 bytes and store the freq counts into rfreq.
            // as we go through the input file, this become cumulative for each "best" table.
            for &symbol in input.iter().take(end as usize).skip(start) {
                rfreq[bt as usize][symbol as usize] += 1;
            }
            // prepare to get the next group of 50 bytes from the input
            start = end + 1;
        } // End of the while loop, we've gone through the entire input one (more) time.
        report(
            opts,
            Chatty,
            format!(
                " pass {}: size is {}, grp uses are {:?}",
                iter + 1,
                total_cost / 8,
                favorites
            ),
        );

        // We will next do improve_code_len_from_freqs on each of the tables we made.
        // This makes actual node trees based off an exaggerated frequency weighting. It
        // will repeatedly flatten that exaggerated weighting until we have all codes
        // 17 or less bits long. This stores the working code lengths into the weight
        // arrays. This makes the next iteration through this better.
        for t in 0..table_count as usize {
            improve_code_len_from_freqs(&mut tables[t], &rfreq[t], eob);
        }
    }
    /*
      4 iterations are now done, and we have good tables and selectors.
      Time to make actual binary codes for reach table. Since we have good lengths,
      we can use the code_from_length function to quickly generate codes.
    */

    // Next are the symbol maps , 16 bit L1 + 0-16 words of 16 bit L2 maps.
    for word in symbol_map {
        bw.out16(word);
    }

    // Symbol maps are followed by a 3 bit number of Huffman trees that exist
    bw.out24((3) << 24 | table_count);

    // Then a 15 bit number indicating the how many selectors are used
    // (how many 50 byte groups are in this block of data)
    bw.out24((15) << 24 | selector_count);

    // Write data depicting which chunks of 50 bytes are decoded by which tables.
    // Given a list of selectors such as [0,2,0,2,1,0], it indicates that bytes
    // 1-50 are decoded by table 0, 51-100 are decoded by table 2, etc.
    for selector in selectors {
        match selector {
            0 => bw.out24((1) << 24),
            1 => bw.out24((2) << 24 | 0x10),
            2 => bw.out24((3) << 24 | 0x110),
            3 => bw.out24((4) << 24 | 0x1110),
            4 => bw.out24((5) << 24 | 0x11110),
            _ => bw.out24((6) << 24 | 0x111110),
        };
    }

    // Now create the huffman codes. We need to convert lengths to code data.
    // (And later we will want to use the BitWriter with those codes also.)
    let mut bw_codes = vec![];

    for table in tables {
        // Create a vec of lengths so we can sort it by length
        let mut len_sym: Vec<(u32, u16)> = vec![];
        for (i, &t) in table.iter().enumerate().take(eob as usize) {
            len_sym.push((t, i as u16));
        }
        len_sym.sort_unstable(); // IF FAILS, use regular sort

        // Get the minimum length in use so we can create the "next code"
        // Next_code contains the 32bit length from len_sym and a 32 bit code.
        // The code is the bitcode (1-17 bits long) used by the huffman coding
        // bitstream.
        let mut next_code: (u32, u32) = (len_sym[0].0, 0);

        // Create a vec that we can push to so we can store the codes.
        let mut sym_codes = vec![];

        // For each len_sym tuple (now sorted by length), increment the next_code by one.
        // When the length changes, do a shift left for each increment and continue.
        for (len, sym) in &len_sym {
            if *len != next_code.0 {
                next_code.1 <<= len - next_code.0;
                next_code.0 = *len;
            }
            // Take a moment to store a version for the bw.out24 format
            // We store the length in the most significant 8 bits.
            // bits: 01234567_XXXXXX_0123456780123456
            // store:length.._blank._17-bit-code.....
            bw_codes.push((*sym, len << 24 | next_code.1));

            // And then push the code for encoding below.
            sym_codes.push((*sym, next_code.1));

            // Increment the next_code.1 counter to generate the next code
            next_code.1 += 1;
        }
        
        // Sym_codes now contains all the bit symbols and codes in this table
        // These now need to be sorted by symbol, not length
        len_sym.sort_by(|a, b| a.1.cmp(&b.1));

        // We first write out the origin code length for this table
        let mut origin = len_sym[0].0;
        //put out the origin as a five bit int
        bw.out24((5) << 24 | origin as u32);

        // All lengths are relative from the last, starting from the origin
        for entry in len_sym.iter() {
            let (l, _) = entry;
            let mut delta = *l as i32 - origin as i32;
            origin = *l;
            loop {
                match delta.cmp(&0) {
                    Ordering::Greater => {
                        bw.out24(0x02_000002);
                        delta -= 1;
                    }
                    Ordering::Less => {
                        bw.out24(0x02_000003);
                        delta += 1;
                    }
                    Ordering::Equal => {
                        break;
                    }
                }
            }
            bw.out24(0x01_000000);
        }
    }
    //view this tree data
    //stream_viewer(bw, 288, 408);
    //stream_viewer(bw, 386, 485);

    // Now send the data. Symbol is basically an index to the codes.
    for symbol in input {
        bw.out24(bw_codes[*symbol as usize].1);
    }
    stream_viewer(bw, 560, 1823);

    // All done
    Ok(())
}

/// Debugging stream viewer
fn stream_viewer(bw: &BitWriter, start: u32, mut end: u32) {
    let stream_end: u32 = (bw.output.len() * 8).try_into().unwrap();
    if start >= stream_end {
        println!(
            "---OOPS --- Stream ends at bit {}, start ({}) is too big.",
            bw.output.len() * 8,
            start
        );
        return;
    };

    if end >= stream_end {
        println!("---OOPS--- Adjusting end to stream end: {}", stream_end - 1);
        end = stream_end - 1;
    };

    let slice = &bw.output[((start) / 8) as usize..((end + 8) / 8) as usize];
    let starting = (start) % 8;
    let ending = end - start;
    let mut progress = 0;
    let mut nibble = 0;
    println!("---Viewing output stream bits {} to {}---", start, end);
    'outer: for byte in slice {
        for i in 0..8 {
            if progress < starting {
            } else {
                if i == 8 {
                    print!("{:b}", byte & 0x1)
                } else {
                    print!("{:b}", byte >> (7 - i) & 0x1)
                }
                nibble += 1;
                if nibble % 4 == 0 {
                    print!(" ");
                }
            }
            progress += 1;
            if nibble == ending {
                break 'outer;
            }
        }
    }
    println!("\n-------------------------------------------");
}

#[test]
fn huf_encode_decode_simple() {
    //let input = "Goofy test".as_bytes();
    //assert_eq!(huf_decode(&huf_encode(input).unwrap()), input)
}
