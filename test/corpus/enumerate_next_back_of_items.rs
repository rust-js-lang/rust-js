// `next_back()` of an enumeration of an array's items made in place:
// the last item and its index, as Rust's `Enumerate::next_back` counts it
// from its length (#395), of a `Vec`'s and of a slice given.
fn last(s: &[u8]) -> Option<(usize, &u8)> {
    s.iter().enumerate().next_back()
}

fn main() {
    let w = vec![10, 20, 30];
    println!("{:?}", w.iter().enumerate().next_back());
    println!("{:?} {:?}", last(&[7, 8]), last(&[]));
    println!("{:?}", w.iter().map(|x| x + 1).next_back());
}
