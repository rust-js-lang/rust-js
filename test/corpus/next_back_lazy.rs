//@ compile-fail: `next_back()` of a lazy
// `next_back()` runs a chain's closures on its last item alone, which a JS
// iterator, stepped from the front, can't: refused.
fn main() {
    let v = vec![1, 2, 3];
    let last = v.iter().filter(|x| {
        println!("saw {x}");
        **x > 1
    }).next_back();
    println!("{last:?}");
}
