use super::symbol_map::encode_sym_map;

/// Encode data using Move To Front transform. Could bring RLE2 into here.
/// Believe major improvements could happen here.
pub fn mtf_encode(raw: &[u8]) -> (Vec<u8>, Vec<u16>) {
    // Create a custom index of the input.
    let mut index = raw.to_owned();
    index.sort_unstable();
    index.dedup();

    // Create the symbol map while we have the input data
    let map = encode_sym_map(&index);

    // ...then do the transform
    let mtf = raw
        .iter()
        .fold((Vec::new(), index), |(mut mtf_v, mut idx), x| {
            let i = idx.iter().position(|c| c == x).unwrap();
            let c = idx.remove(i); //might be faster to swap everything going to 0
            idx.insert(0, c);
            mtf_v.push(i as u8);
            (mtf_v, idx)
        })
        .0;
    (mtf, map)
}

/// Decode data using Move To Front transform. Could bring RLE2 into here.
/// Decoding requires a sorted symbol map index.
pub fn mtf_decode(raw: &[u8], index: Vec<u8>) -> Vec<u8> {    
    raw.iter()
        .fold((Vec::new(), index), |(mut mtf_v, mut s), x| {
            mtf_v.push(s[*x as usize]);
            let tmp = s.remove(*x as usize);
            s.insert(0, tmp);
            (mtf_v, s)
        })
        .0
        .into_iter()
        .map(|c| c as u8)
        .collect::<Vec<u8>>()
}

#[test]
fn simple_encode() {
    let input = "Baa baa".to_string().as_bytes().to_vec();
    let output = &[1, 2, 0, 2, 3, 2, 0];
    let (x, _) = mtf_encode(&input);
    assert_eq!(x, output);}

#[test]
fn mtf_encode_with_key() {
    let input = [164, 11, 0, 0, 34, 97, 97, 32, 98, 97, 97].to_vec();
    let output = &[6, 2, 2, 0, 4, 5, 0, 5, 6, 2, 0];
    let (x, _) = mtf_encode(&input);
    assert_eq!(x, output);
}

#[test]
fn mtf_encode_from_book() {
    let input = "bbyaeeeeeeafeeeybzzzzzzzzzyz".as_bytes().to_vec();
    let output = &[
        1, 0, 4, 2, 3, 0, 0, 0, 0, 0, 1, 4, 2, 0, 0, 3, 4, 5, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1,
    ];
    let (x, _) = mtf_encode(&input);
    assert_eq!(x, output);
}
