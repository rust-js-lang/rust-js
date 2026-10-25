//@ compile-fail: as a `DoubleEndedIterator`, which rust-js can't step from its back yet
// A `filter_map` held in a variable that a map stepped from its back steps
// (#218): no form of it steps from its back where it's made, so it's
// refused, not stepped as a JS iterator with no back.
fn main() {
    let v = vec![1, 2, 3, 4, 5, 6];
    let tripled = v.iter().filter_map(|x| if x % 2 == 0 { Some(x * 3) } else { None });
    let mut more = tripled.map(|x| x + 1);
    println!("{:?} {:?}", more.next_back(), more.next());
}
