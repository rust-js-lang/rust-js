//@ run-fail: range end index 9 out of range for slice of length 3
// `extract_if` checks its range as it's made, before anything asks it for an item.
fn main() {
    let mut v = vec![1, 2, 3];
    let end = v.len() + 6;
    let taken = v.extract_if(..end, |_| true);
    println!("made");
    drop(taken);
}
