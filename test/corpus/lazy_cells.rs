// A `LazyCell` or `LazyLock` makes its value the first time it's used, as
// Rust's does: a `static` one, read through `Deref`, forced, and changed.

use std::cell::LazyCell;
use std::collections::HashMap;
use std::sync::LazyLock;

static TABLE: LazyLock<HashMap<&str, u32>> = LazyLock::new(|| {
    println!("making the table");
    HashMap::from([("one", 1), ("two", 2)])
});

static NAMES: LazyLock<Vec<String>> = LazyLock::new(|| vec!["a".to_string(), "b".to_string()]);

fn lookup(name: &str) -> Option<u32> {
    TABLE.get(name).copied()
}

fn main() {
    println!("before");
    println!("{:?} {:?} {}", lookup("one"), lookup("three"), TABLE.len());
    println!("{} {:?}", NAMES.len(), *NAMES);

    let mut calls = 0;
    let lazy = LazyCell::new(|| {
        calls += 1;
        "made".to_string()
    });
    println!("{:?}", LazyCell::get(&lazy));
    println!("{} {}", *lazy, lazy.len());
    println!("{:?} {}", LazyCell::get(&lazy), LazyCell::force(&lazy));
    drop(lazy);
    println!("{calls}");

    let mut count = LazyCell::new(|| 40);
    println!("{:?}", LazyCell::get_mut(&mut count));
    *count += 1;
    *LazyCell::force_mut(&mut count) += 1;
    if let Some(n) = LazyCell::get_mut(&mut count) {
        *n += 1;
    }
    println!("{} {:?}", *count, count);

    let mut list: LazyCell<Vec<u8>> = LazyCell::default();
    println!("{list:?}");
    list.push(1);
    LazyCell::force_mut(&mut list).push(2);
    println!("{list:?} {list:#?}");

    // `()` looks like `None` in JS: what's made is still there.
    let unit = LazyCell::new(|| println!("unit"));
    println!("{:?}", LazyCell::get(&unit));
    LazyCell::force(&unit);
    println!("{:?} {unit:?}", LazyCell::get(&unit));
}
