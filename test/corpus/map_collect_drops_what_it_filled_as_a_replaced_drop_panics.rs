// A `collect()` into a `HashMap` of two items with one key replaces the
// first value, dropping it; where that drop panics, the map it was filling,
// `from_iter`'s own, holding the second, is dropped as the panic unwinds,
// as std's frame drops it (#1646). Where it wasn't, the second leaked.
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
#[derive(PartialEq, Eq, Hash)]
struct Key(u8);
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud {} panicking={}", self.0, std::thread::panicking());
        if self.0 == 1 && !std::thread::panicking() {
            panic!("drop of 1");
        }
    }
}
fn main() {
    let r = catch_unwind(AssertUnwindSafe(|| {
        let km: HashMap<Key, Loud> = [(Key(0), Loud(1)), (Key(0), Loud(2))].into_iter().collect();
        km.len()
    }));
    println!("{}", r.is_err());
}
