use super::{duval::rotate_duval, sais::build_suffix_array};

/// Computes the Burrows-Wheeler-Transform of the input data without a sentinel value.
/// It uses the duval algorithm to provide a lexicographically minimal rotation of the input string
/// and passes this to the SAIS algorithm. The rotation makes sure that the BWT is computed
/// correctly because the rotation is lexicographically minimal.
pub fn bwt(string: &[u8]) -> (usize, Vec<u8>) {
    let (rotated, shift) = rotate_duval(&string);

    let entries = build_suffix_array(&rotated);
    let len = string.len();
    let bwt = entries
        .iter()
        .filter(|x| x.index < len)
        .map(|x| {
            let index = if x.index > 0 { x.index - 1 } else { len - 1 };
            rotated[index]
        })
        .collect::<Vec<_>>();
    let orig_ptr = entries
        .iter()
        .filter(|x| x.index < len)
        .enumerate()
        .find(|(_, x)| x.index == (len - shift) % len)
        .unwrap()
        .0;
    (orig_ptr, bwt)
}

