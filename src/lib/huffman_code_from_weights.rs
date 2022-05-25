use super::huffman::{Node, NodeData};

/// Improve a slice of Huffman codes lengths (u8) using a slice of  
/// codes, symbol weights, and knowlege of how many symbols are valid. Returns depth.
pub fn improve_code_len_from_weights<'a>(
    codes: &'a mut [u32],  //[u32; 258]
    sym_weight: &'a [u32], //[u32; 258]
    eob: u16,              //symbol marking last valid byte in the above slices
) -> &'a [u32] {
    // Assign initial weights to each symbol based on the weight
    // If the weight was 0, put 1 otherwise put weight * 256 ( << 8 )
    // Note: We need to start with one weight in the array for the sorting to work properly.
    // Using indexing instead of pushing for speed.
    let mut weight: Vec<(u32, u16)> = vec![(0, 0); eob as usize + 2];
    for (i, f) in sym_weight.iter().enumerate().take(eob as usize + 1) {
        weight[i + 1] = ((if f == &0 { 256 } else { f << 8 }, i as u16));
        // Do a Julian style approximate fast 'sort'. (sort_unstable doesn't work as well)
        push_big_down(&mut weight, i);
    }

    // We need to make codes of 17 bits or less. If we can't, we will adjust the weights and try again.
    'outer: loop {
        // Turn the array into a tree
        let mut tree: Vec<Node> = weight
            .iter()
            .skip(1)
            .map(|&(f, m)| Node::new(f, 0, NodeData::Leaf(m)))
            .collect();

        // reverse the tree so we can we pop elements
        tree.reverse();

        // ...then pare it down to one single node with child nodes - keep it sorted.
        while tree.len() > 1 {
            let left_child = tree.pop().unwrap();
            let right_child = tree.pop().unwrap();
            tree.push(Node::new(
                add_weights(left_child.weight, right_child.weight),
                left_child.depth.max(right_child.depth) + 1,
                NodeData::Kids(Box::new(left_child), Box::new(right_child)),
            ));
            // Keep the leaves sorted by weight so we pop elements correctly.
            tree.sort_unstable_by(|a, b| b.weight.cmp(&a.weight));
        }

        // If the tree depth <= 17 copy the new depths back into the code table.
        // Otherwise adjust weights and try again.
        // NOTE: THERE MAY BE A FASTER WAY TO DO THIS OTHER THAN RECURSING A TREE
        if tree[0].depth <= 17 {
            let mut leaves = vec![];
            return_leaves(&tree[0], 0, &mut leaves);

            for (idx, len) in leaves {
                codes[idx as usize] = len as u32;
            }
            // Overwrite the codes and return the improved list.
            break 'outer codes;
        } else {
            // Adjust weights by dividing each weight by 2 and adding 1
            // This "flattens" the node tree. Then go try this again.
            for item in weight.iter_mut().take(eob as usize + 1) {
                let mut j = item.0 >> 8;
                j = 1 + (j / 2);
                item.0 = j << 8;
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
        }
    };
}

/// Julian's version of weight adding for parent nodes
#[inline(always)]
fn add_weights(a: u32, b: u32) -> u32 {
    let weigh_mask: u32 = 0xffffff00;
    let depth_mask: u32 = 0x000000ff;
    ((a & weigh_mask) + (b & weigh_mask)) | (1 + (a & depth_mask).max(b & depth_mask))
}

///  Julian slide sort. Gets things in the right direction but not fully sorted.
pub fn push_big_down<T: std::cmp::PartialOrd + Clone>(vec: &mut Vec<T>, mut idx: usize) {
    if idx == 0 {
        return;
    }
    idx;
    let tmp = vec[idx].clone();
    while vec[idx] < vec[idx >> 1] {
        vec.swap(idx, idx >> 1);
        idx >>= 1;
    }
    vec[idx] = tmp;
}
