// `convert::identity(x)` is `x`, as a value and as a function given to an
// adapter: what it's given, unchanged.
use std::convert::identity;

fn main() {
    let words = vec![Some("a"), None, Some("b")];
    let kept: Vec<&str> = words.into_iter().flatten().map(identity).collect();
    let n = identity(41) + 1;
    let nested: Vec<u32> = vec![vec![1, 2], vec![3]].into_iter().flat_map(identity).collect();
    println!("{kept:?} {n} {nested:?}");
}
