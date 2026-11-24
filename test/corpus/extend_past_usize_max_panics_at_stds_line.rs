//@ run-fail: capacity overflow
// A `Vec`'s `extend` of an inclusive range of every `usize`, uncaught:
// std's hook names its `extend_trusted`'s line, where JS pushed to its
// arrays' longest and threw its own `RangeError` at `<unknown>` (#2511).
fn main() {
    println!("start");
    let mut v = vec![1usize];
    v.extend(0..=usize::MAX);
    println!("{}", v.len());
}
