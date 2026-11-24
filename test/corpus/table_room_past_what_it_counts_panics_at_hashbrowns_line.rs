//@ run-fail: Hash table capacity overflow
// A hash table's room past what its buckets can count, uncaught: std's
// hook names hashbrown's line, its `Fallibility::capacity_overflow`'s
// `panic!`, where the runtime threw a plain `Error` the hook named
// `<unknown>` (#2487).
use std::collections::HashMap;

fn main() {
    println!("start");
    let table: HashMap<u64, u64> = HashMap::with_capacity(usize::MAX);
    println!("{}", table.len());
}
