// A slice's `to_owned()` is its `to_vec()`, as percent-encoding's
// `initial_bytes[..n].to_owned()`; and `to_vec()` of an array made where
// it's called is that array, of items that need no clone (ADR 0153).
#[derive(Clone, Debug)]
struct Token(String);

fn decoded(bytes: &[u8], n: usize) -> Vec<u8> {
    let mut decoded = bytes[..n].to_owned();
    decoded.push(b'!');
    decoded
}

fn main() {
    let bytes = *b"abc%20";
    println!("{:?} {:?}", decoded(&bytes, 3), bytes);
    let tokens = vec![Token("a".into()), Token("b".into())];
    let mut copied = tokens[1..].to_owned();
    copied[0].0.push('!');
    println!("{:?} {:?}", tokens, copied);
    let doubled: Vec<u32> = [1, 2].iter().map(|n| n * 2).collect::<Vec<_>>()[..].to_vec();
    println!("{doubled:?}");
}
