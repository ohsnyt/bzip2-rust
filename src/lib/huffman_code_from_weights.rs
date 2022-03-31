use log::{trace, warn, debug};
use super::huffman::{Node, NodeData};

/// Improve a slice of Huffman codes lengths (u8) using a slice of  
/// codes, symbol weights, and knowlege of how many symbols are valid
/// STILL NOT SYNCING WITH JULIAN'S IMPLEMENTATION. 17 March 2022.
/// Should this be implemented with a BinaryHeap?
pub fn improve_code_len_from_weights<'a>(
    codes: &'a mut [u32],
    sym_weight: &'a [u32],
    eob: u16,
) -> &'a [u32] {
    // Assign initial weights to each symbol based on the weight
    // If the weight was 0, put 1 otherwise put weight * 256 ( << 8 )
    let mut weight: Vec<(u32, u16)> = vec![];
    weight.push((0, 0));
    //weight.push((32, 0));
    for (i, f) in sym_weight.iter().enumerate().take(eob as usize + 1) {
        weight.push((if f == &0 { 256 } else { f << 8 }, i as u16));
        // Do a Julian style approximate fast 'sort'.
        push_big_down(&mut weight);
        // NOTE: I did use weight.sort_unstable(), but I'm trying to duplicate the odd behavior Julian has.
    }

    // We will try to make codes of 17 bits or less. If we can't, we will
    // cut the weights down and try again.
    'outer: loop {
        // Turn the array into a tree
        let mut tree = Vec::new();
        for (f, m) in weight.iter().skip(1) {
            tree.push(Node::new(*f, 0, NodeData::Leaf(*m)));
        }

        // reverse the tree because we pop rather than pick from the front of the array
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
            // Do a Julian style approximate fast 'sort'.
            // let tree_clone = tree.clone();
            //push_big_up(&mut tree); // needed???
            // if tree != tree_clone {
            //     debug!("Hmmm")
            // } else {
            //     debug!("ok")
            // }
            // NOTE: I did have a true sort, but I'm trying to duplicate the odd behavior Julian has.
            tree.sort_by(|a, b| b.weight.cmp(&a.weight));
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
            trace!("symbol {}, depth {}", mtf, depth);
        }
    };
}

/// Julian's version of weight adding for parent nodes
fn add_weights(a: u32, b: u32) -> u32 {
    let weigh_mask: u32 = 0xffffff00;
    let depth_mask: u32 = 0x000000ff;
    ((a & weigh_mask) + (b & weigh_mask)) | (1 + (a & depth_mask).max(b & depth_mask))
}

///  Julian slide sort. Gets things in the right direction but not fully sorted.
pub fn push_big_up(vec: &mut Vec<Node>) {
     let mut idx = vec.len() - 1;
    while idx > 0 && vec[idx] > vec[idx - 1] {
        vec.swap(idx, idx - 1);
        idx -= 1;
    }
    //vec[idx] = tmp;
    // let end = vec.len() - 1;
    // loop {
    //     let mut y = end >> 1;
    //     if y == 0  {
    //         break;
    //     };
    //     if (y > 0) && (vec[y - 1] > vec[y]) {
    //         y -= 1;
    //     }
    //     if vec[0] < vec[y] {
    //         break;
    //     };
    //     vec.swap(end, y);
    // }
}

///  Julian slide sort. Gets things in the right direction but not fully sorted.
pub fn push_big_down<T: std::cmp::PartialOrd + Clone>(vec: &mut Vec<T>) {
    let mut idx = vec.len() - 1;
    let tmp = vec[idx].clone();
    while vec[idx] < vec[idx >> 1] {
        vec.swap(idx, idx >> 1);
        idx >>= 1;
    }
    vec[idx] = tmp;
}
