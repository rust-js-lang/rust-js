// A `&mut` to a struct in a `Vec`, given straight to generic code's `&mut T`:
// an in-place cell of the item, `$cellOf($index(v, 0))`, whose write
// changes it in place (ADR 0454, amended), where its box, copied back to a
// place that isn't fixed, was refused.
fn set<T>(t: &mut T, u: T) {
    *t = u;
}
struct P {
    x: i32,
}
fn main() {
    let mut v = vec![P { x: 1 }];
    set(&mut v[0], P { x: 2 });
    println!("{}", v[0].x);
}
