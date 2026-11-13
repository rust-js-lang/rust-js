//@ run-fail: none
// A `filter_map` over items holding a `u64`, aligned to 8 on wasm32 and
// natively alike, whose closure panics, collected into a `Vec` of `D`,
// aligned to 4: std collects it otherwise than in place, so what's left of
// its source, `D(9)`, drops before what it collected, `D(7)`, as
// `spec_extend` takes the iterator by value into its own frame (#1651).
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Wide(Vec<D>, u64);
fn firsts<I: Iterator<Item = Wide>>(items: I) -> Vec<D> {
    items
        .filter_map(|wide| {
            if wide.1 == 0 {
                panic!("none");
            }
            wide.0.into_iter().next()
        })
        .collect()
}
fn main() {
    let items = vec![Wide(vec![D(7), D(8)], 1), Wide(vec![], 0), Wide(vec![D(9)], 1)];
    let _ = firsts(items.into_iter());
}
