use super::bitwriter::BitWriter;
use super::rle2::MTF;
use std::{
    collections::{BTreeMap, HashMap},
    io::Error,
};

#[derive(Eq, PartialEq, Debug)]
enum NodeData {
    Kids(Box<Node>, Box<Node>),
    Leaf(MTF),
}

#[derive(Eq, PartialEq, Debug)]
struct Node {
    frequency: u32,
    node_data: NodeData,
}
impl Node {
    fn new(frequency: u32, node_data: NodeData) -> Node {
        Node {
            frequency,
            node_data,
        }
    }
}

/// Encode MTF data using the Huffman algorithm, We need the bwt key & crc also
pub fn huf_encode(input: &[MTF], bw: &mut BitWriter, symbol_map: Vec<u16>) -> Result<(), Error> {
    // for el in input {
    //     match el {
    //         MTF::Sym(8) => println!("Got symbol 8"),
    //         MTF::EOB => println!("Got EO8"),
    //         _ => {},
    // }}

    // First calculate frequencies of u8s for the Huffman tree
    // Use a hashmap to simplify things
    let mut map: HashMap<MTF, u32> = HashMap::new();
    for byte in input {
        let e = map.entry(*byte).or_insert(1);
        *e += 1;
    }

    //Now create the huffman tree - using a vec (probably not best, but binary heap didn't work)
    let mut vec = Vec::new();
    for (m, f) in map.iter() {
        vec.push(Node::new(*f, NodeData::Leaf(*m)))
    }

    // Oh, (NEW): Fill in "empty" symbols removed by the MTF. Give them a frequency of 0
    // Find every Sym leaf, and create a vec of the numbers
    let mut sym_vec = Vec::new();
    for node in &vec {
        let Node {
            frequency: _,
            node_data,
        } = node;
        match node_data {
            NodeData::Leaf(m) => match m {
                MTF::Sym(n) => sym_vec.push(n),
                _ => {}
            },
            _ => {}
        }
    }
    // Make a new vec of the missing symbols.
    let mut extras: Vec<Node> = Vec::new();
    sym_vec.sort();
    for n in 2..**sym_vec.last().unwrap() {
        if !sym_vec.contains(&&n) {
            extras.push(Node::new(0, NodeData::Leaf(MTF::Sym(n))))
        }
    }
    vec.extend(extras); //what a pain the butt... but it worked.

    // Sort the vec descending.
    vec.sort_by(|a, b| b.frequency.cmp(&a.frequency));

    // ...then pare it down to one single node with child nodes - keep it sorted.
    while vec.len() > 1 {
        let left_child = vec.pop().unwrap();
        let right_child = vec.pop().unwrap();
        vec.push(Node::new(
            left_child.frequency + right_child.frequency,
            NodeData::Kids(Box::new(left_child), Box::new(right_child)),
        ));
        vec.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    }
    //println!("Tree\n{:?}", vec);

    // Starting with the most frequent character, walk down tree and generate bit codes
    // This is recursive. It returns the u8, the bit length of the code, the code as a u32.
    // NOTE: to get the actual count of codes, you need to ask for codes.len()+3 (enum thing)
    // SEEMS LIKE THIS DOESN'T LIKE 8, 16 AND SIMILAR NUMBERS.
    let mut codes = BTreeMap::new();
    gen_codes(vec.first().unwrap(), vec![0u8; 0], &mut codes);

    for code in &codes {
        println!("{:?}", code)
    }

    // Start generating this block's compressed data stream. (The file header is written elsewhere.)
    //NOTE-- The crc_list must be held by whatever calls this.
    //       This lisht grows with each block written.
    //       It is created here as a placeholder during development where I test small data sets.
    // let mut crc_list = vec![]; // used to keep track of compressed block crcs

    // Next are the symbol maps , 16 bit L1 + 0-16 words of 16 bit L2 maps.
    for word in symbol_map {
        bw.out16(word);
    }

    // Symbol maps are followed by a 3 bit number of Huffman trees that exist
    // For development, I'm creating one map only. Specification say we must use 2-6.
    let trees = vec![&codes, &codes];
    bw.out24((3 as u32) << 24 | (trees.len() as u32)); // ensure h_trees is a u32

    // Then a 15 bit number indicating the array depicting which symbols are decoded by
    // which tables. Given a list such as [0,2,0,2,1,0], it indicates that symbols 1-50
    // are decoded by table 0, 51-100 are decoded by table 2, etc.
    // For development, everthing uses table 0 as defined in the next line. BUT SOMEHOW
    // IT COMES OUT TO A BINARY 2.
    // MY CODE IS SCREWED UP. IF IT IS 15 BITS, IT SHOULD END IN 010. AS IT IS, I HAVE THAT
    //010 COMING FROM THE NEXT BIT OF BOGUS INFO.
    //let num_sels: Vec<u8> = vec![0, 0, 0, 0, 0];
    bw.out24((15 as u32) << 24 | 2); //(num_sels_encode(num_sels)));

    // I am semi-clueless. THE BOOK HAS ANOTHER THREE BIT 2 HERE. I'LL ADD IT.
    // I THINK it means that there are two trees specified next
    bw.out24(0x03_000002);

    //-------gotta push out the trees, then the data here
    for tree in trees {
        let mut first_l = 0;
        if let Some((s, (l, m))) = tree.iter().next() {
            first_l = *l;
            println!("\n{:?}, length {}, code {:08b}", s, l, m);
        };
        let mut origin = first_l as i32;
        //put out the origin as a five bit int
        bw.out24((5 as u32) << 24 | origin as u32); //(num_sels_encode(num_sels)));
                                                    // all subsequent lengths are computed from the origin
        for entry in tree.iter().skip(1) {
            let (s, (l, m)) = entry;
            let mut delta = *l as i32 - origin;
            println!("{:?}, length {}, code {:08b}", s, l, m);
            origin = *l as i32;
            loop {
                if delta > 0 {
                    bw.out24(0x02_000002);
                    delta -= 1;
                } else if delta < 0 {
                    bw.out24(0x02_000003);
                    delta += 1;
                } else if delta == 0 {
                    //bw.out24(0x01_000000);  // do nothing because we will push a zero below.
                }
                if delta == 0 {
                    break;
                };
            }
            bw.out24(0x01_000000);
        }
    }
    //view this tree data
    stream_viewer(bw, 286, 385);
    stream_viewer(bw, 386, 485);

    // Now the data
    for symbol in input {
        match symbol {
            MTF::RUNA => {
                let (l, c) = codes.get(&MTF::RUNA).unwrap();
                bw.out24((*l as u32) << 24 | c);
            }
            MTF::RUNB => {
                //let x =
                let (l, c) = codes.get(&MTF::RUNB).unwrap();
                bw.out24((*l as u32) << 24 | c);
            }
            MTF::Sym(n) => {
                let (l, c) = codes.get(&MTF::Sym(*n)).unwrap();
                bw.out24((*l as u32) << 24 | c);
            }
            MTF::EOB => {
                let (l, c) = codes.get(&MTF::EOB).unwrap();
                bw.out24((*l as u32) << 24 | c);
            }
        }
    }
    //view this tree data
    //stream_viewer(bw, 408, 823);

    // Calculate and store this compressed block CRC
    //crc_list.push(bz_stream_crc(&crc_list)); // SOMETHING NOT RIGHT HERE

    // The block stream footer starts with a 48 bit magic number (sqrt pi)
    //bw.flush(); //just for testing
    bw.out24(0x18_177245); // magic bits  1-24
    bw.out24(0x18_385090); // magic bits 25-48
                           //bw.out32(*crc_list.last().unwrap()); // output this CRC
                           //bw.flush(); // make data byte aligned and we are done
                           //view this tree data
    stream_viewer(bw, 823, 1500);

    Ok(())
}

// Walk down every branch of tree to get codes for every final leaf ------
// If the node has two child nodes, it then pushes a 0 to every left node and goes to look
// for more left nodes. If the node is a terminal leaf in the tree, it inserts the bit_data
// it has built. This then recurses up. When recursing up, it goes on to push a 1 onto the
// right node bit_data and recurses again
/// Generate bit codes for a Huffman tree
fn gen_codes(node: &Node, bit_data: Vec<u8>, codes: &mut BTreeMap<MTF, (u8, u32)>) {
    //println!("gen_code for {:?}", &node);
    match &node.node_data {
        NodeData::Kids(ref left_child, ref right_child) => {
            let mut left_prefix = bit_data.clone();
            left_prefix.push(0);
            gen_codes(left_child, left_prefix, codes);

            let mut right_prefix = bit_data;
            right_prefix.push(1);
            gen_codes(right_child, right_prefix, codes);
        }
        NodeData::Leaf(mtf) => {
            let depth = bit_data.len() as u8 + 1;
            let mut code: u32 = 0;
            for bit in bit_data {
                code |= bit as u32;
                code <<= 1;
            }
            codes.insert(*mtf, (depth, code));
            //println!("Generated {} bit code {:08b} for {:?}", depth, code, mtf);
        }
    }
}

/// For decoding, store codes and bitdepth in a hashmap. This should allow for fast decoding.
pub fn huf_decode(data: &[u8]) -> Vec<u8> {
    let code_count =
        u32::from_le_bytes(data[0..4].try_into().expect("Error reading huffman codes")) as usize;

    let mut codes: HashMap<u32, u8> = HashMap::new();
    let mut pos = 4;
    for i in 1..code_count + 1 {
        let symbol = data[pos];
        let code = u32::from_le_bytes(
            data[pos + 1..pos + 5]
                .try_into()
                .expect("Error reading huffman codes"),
        );
        //println!("Iteration {}. Got code for {} of {:#32b}", i, symbol, code);
        pos = i * 5 + 4;
        codes.insert(code, symbol);
    }

    let mut curr_code: u32 = 0; // used to match our bit demarked u32 index system
    let mut data_out: Vec<u8> = Vec::new();
    //    let code_list: Vec<(u8, u32)> = codes.iter().map(|(&c, &s)| (s, c)).collect();
    let mut depth = 0;
    //pos = code_count * 5 + 4;
    //reading through each byte of compressed data...
    for i in pos..data.len() {
        let mut byte = data[i];
        //unpack the byte, bit by bit, checking for a match with each bit added
        for _ in 0..8 {
            depth += 1;
            curr_code <<= 1;
            curr_code |= (byte >> 7) as u32;
            byte <<= 1;
            // println!(
            //     "DC: Depth is {}, code is {:b}, combined is {:#032b}",
            //     depth,
            //     curr_code,
            //     curr_code | (depth << 26)
            // );
            if codes.contains_key(&(curr_code | (depth << 26))) {
                let c = *codes.get(&(curr_code | (depth << 26))).unwrap();
                //println! {"{}", *codes.get(&(curr_code | (depth << 26))).unwrap()};
                data_out.push(c);
                curr_code = 0;
                depth = 0;
            }
        }
        pos += 1;
    }
    //println! {"{:?}",data_out};
    data_out
}

/// Converts the number of trees into a u8 with bits set for the maps used.
fn map_h_tree_use(h_trees: u8) -> u8 {
    let mut treecode: u8 = 0;
    for _ in 1..h_trees {
        treecode |= 0x01;
        treecode <<= 1;
    }
    treecode <<= 1;
    treecode
}

/// Encode NumSels (num_sels) vec into BZIP 15 bit codes, returned as u32
/// (Seems like this returns bad info if more than 3 tables are used.) Currently not developed
fn num_sels_encode(v: Vec<u8>) -> u32 {
    let mut num_sels_encoded: u32 = 0;
    let mut total = 0; //used for print statement below
    for i in v {
        let code = match i {
            0 => 0x0,
            1 => 0x10,
            2 => 0x110,
            3 => 0x1110,
            4 => 0x11110,
            _ => 0x111110,
        };
        num_sels_encoded <<= i + 1;
        num_sels_encoded |= code;
        total += i + 1; // remove this after testing
        {
            print!(
                "NumSel code is {:015b}, using {} of 15 bits for the NumSels field.\r",
                code, total
            )
        };
    }
    println!("\n");
    num_sels_encoded
}

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
                    print!("{:b}", byte >> 7 - i & 0x1)
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
    let input = "Goofy test".as_bytes();
    //    assert_eq!(huf_decode(&huf_encode(input).unwrap()), input)
}
