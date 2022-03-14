use super::huffman::{Node, NodeData};

/// Improve a slice of Huffman codes lengths (u8) using a slice of  
/// codes, symbol frequencies, and knowlege of how many symbols are valid
/// BROKEN as of 11 March 2022. ds.
pub fn improve_code_len_from_freqs<'a>(
    codes: &'a mut [u32],
    sym_freq: &'a [u32],
    eob: u16,
) -> &'a [u32] {
    // Assign initial weights to each symbol based on the frequency
    // A freq of 0 = weight of 1, everything else is freq * 256 (shift left 8x)
    let mut weight: Vec<(u32, u16)> = vec![];
    for (i, f) in sym_freq.iter().enumerate().take(eob as usize) {
        weight.push(((f << 8).max(1), i as u16));
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

        // ...then pare it down to one single node with child nodes - keep it sorted.
        while tree.len() > 1 {
            let left_child = tree.pop().unwrap();
            let right_child = tree.pop().unwrap();
            tree.push(Node::new(
                left_child.frequency + right_child.frequency,
                left_child.depth.max(right_child.depth)+1,
                NodeData::Kids(Box::new(left_child), Box::new(right_child)),
            ));
            tree.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        }

        // If the tree depth <= 17 copy the new depths back into the code table.
        // Otherwise adjust weights and try again.
        if tree[0].depth <= 17 {
            let mut vec = vec![];
            return_leaves(&tree[0], 0, &mut vec);

            for (idx, len) in vec {
                codes[idx as usize] = len as u32;
            }
            break 'outer codes;
        } else {
            // Adjust weights (frequencies) by dividing each frequency by 2 and adding 1
            // This "flattens" the node tree. Then go try this again.
            for item in weight.iter_mut().take(eob as usize).skip(1) {
                let mut j = item.1 >> 8;
                j = 1 + (j / 2);
                item.1 = j << 8;
            }

            // This version probably does more of the map than we need.
            //weight = weight.iter().map(|(w, s)| (w / 2.min(1), *s)).collect();
        }
    }
    // Overwrite the codes and return the improved list.
}

fn return_leaves(node: &Node, depth: u8,  vec: &mut Vec<(u16, u8)>)  {
    let nd = &node.node_data;
    match nd {
        NodeData::Kids(ref left_child, ref right_child) => {
            return_leaves(left_child, depth+1, vec);
            return_leaves(right_child, depth+1, vec);
        }
        NodeData::Leaf(mtf) => {
            vec.push((*mtf, depth));
        }
    };
}
