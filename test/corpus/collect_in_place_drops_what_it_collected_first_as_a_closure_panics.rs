//@ run-fail: none
// A `filter_map` over `Option<D>` items whose closure panics, collected into
// a `Vec`: std collects it in place, its items as aligned as `D` and no
// smaller, on wasm32 and natively alike, so what it collected, `D(7)`, drops
// before what's left of its source, `D(9)`, as std's `InPlaceDrop` guard is
// dropped by `try_fold`'s frame (#1651).
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn firsts<I: Iterator<Item = Option<D>>>(items: I) -> Vec<D> {
    items
        .filter_map(|item| {
            if item.is_none() {
                panic!("none");
            }
            item
        })
        .collect()
}
fn main() {
    let _ = firsts(vec![Some(D(7)), None, Some(D(9))].into_iter());
}
