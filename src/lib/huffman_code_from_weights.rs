use log::{debug, trace};

use super::huffman::{Node, NodeData};

/// Improve a slice of Huffman codes lengths (u8) using a slice of  
/// codes, symbol frequencies, and knowlege of how many symbols are valid
/// STILL NOT SYNCING WITH JULIAN'S IMPLEMENTATION. 17 March 2022.
pub fn improve_code_len_from_freqs<'a>(
    codes: &'a mut [u32],
    sym_freq: &'a [u32],
    eob: u16,
) -> &'a [u32] {
    // Assign initial weights to each symbol based on the frequency
    // If the freq was 0, put 1 otherwise put freq * 256 ( << 8 )
    let mut weight: Vec<(u32, u16)> = vec![];
    for (i, f) in sym_freq.iter().enumerate().take(eob as usize + 1) {
        weight.push((if f == &0 { 1 } else { f << 8 }, i as u16));
    }

    // sort this by the weight
    weight.sort_unstable();

    // We will try to make codes of 17 bits or less. If we can't, we will
    // cut the weights down and try again.
    'outer: loop {
        // Turn the array into a tree
        let mut tree = Vec::new();
        for (f, m) in weight.iter() {
            tree.push(Node::new(*f, 0, NodeData::Leaf(*m)));
        }

        // sort the tree by frequency before the next step
        tree.sort_by(|a, b| b.frequency.cmp(&a.frequency));

        // ...then pare it down to one single node with child nodes - keep it sorted.
        while tree.len() > 1 {
            let left_child = tree.pop().unwrap();
            let right_child = tree.pop().unwrap();
            tree.push(Node::new(
                add_weights(left_child.frequency, right_child.frequency),
                left_child.depth.max(right_child.depth) + 1,
                NodeData::Kids(Box::new(left_child), Box::new(right_child)),
            ));
            tree.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        }

        // If the tree depth <= 17 copy the new depths back into the code table.
        // Otherwise adjust weights and try again.
        if tree[0].depth <= 17 {
            let mut leaves = vec![];
            return_leaves(&tree[0], 0, &mut leaves);

            for (idx, len) in leaves {
                codes[idx as usize] = len as u32;
            }
            // Overwrite the codes and return the improved list.
            break 'outer codes;
        } else {
            debug!("Lengths exceeded 17 bits... adjusting weights.");
            // Adjust weights (frequencies) by dividing each frequency by 2 and adding 1
            // This "flattens" the node tree. Then go try this again.
            for item in weight.iter_mut().take(eob as usize + 1).skip(1) {
                let mut j = item.1 >> 8;
                j = 1 + (j / 2);
                item.1 = j << 8;
            }
        }
    }
    // Overwrite the codes and return the improved list.
}

/// Walk the tree and return in "leaves" how far (deep) from the root node each leaf is.
/// Depth is the same as the code length, and will be used to create actual codes later.
fn return_leaves(node: &Node, depth: u8, leaves: &mut Vec<(u16, u8)>) {
    // Get the Node Data
    let nd = &node.node_data;
    // If it is node with kids, recurse. Otherwise push the terminal leaf symbol and depth.
    match nd {
        NodeData::Kids(ref left_child, ref right_child) => {
            return_leaves(left_child, depth + 1, leaves);
            return_leaves(right_child, depth + 1, leaves);
        }
        NodeData::Leaf(mtf) => {
            leaves.push((*mtf, depth));
            trace!("symbol {}, depth {}", mtf, depth);
        }
    };
}

/// Julian's version of weight adding for parent nodes
fn add_weights(a: u32, b: u32) -> u32 {
    (a + b) | (1 + a.max(b))
}
