// `size_hint()` of an adapter made in place that's counted where it's
// made, a slice's `skip()` or `take()`: its length both ways, as Rust's
// `ExactSizeIterator` hints it (#156).
fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    println!("{:?}", v.iter().skip(1).size_hint());
    println!("{:?}", v.iter().take(2).size_hint());
    println!("{:?}", v.iter().skip(9).size_hint());
    println!("{:?}", v.iter().map(|x| x + 1).size_hint());
    println!("{:?}", v.iter().skip(1).take(3).size_hint());
}
