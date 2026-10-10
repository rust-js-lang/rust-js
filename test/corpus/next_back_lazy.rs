//@ compile-fail: `next_back()` of a lazy
// `next_back()` runs a chain's closures on its last item alone, which a JS
// iterator, stepped from the front, can't: refused.
fn main() {
    let v = vec![1, 2, 3];
    let last = v.iter().inspect(|x| println!("saw {x}")).next_back();
    println!("{last:?}");
}
