// A function an `Option`'s or a `Result`'s method leaves unrun, `map`'s of a
// `None`, `unwrap_or_else`'s of a `Some`, is dropped by it, with what it
// holds, as Rust's std drops it; one it runs drops what it holds as it ends
// (ADR 0469).
use std::collections::HashMap;

struct Loud(u32);

impl Loud {
    fn get(&self) -> u32 {
        self.0
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn or_else<F: FnOnce() -> u32>(value: Option<u32>, f: F) -> u32 {
    value.unwrap_or_else(f)
}

fn main() {
    let some = Some(5);
    let none: Option<u32> = None;
    let a = Loud(1);
    println!("map {:?}", none.map(move |x| x + a.get()));
    let a = Loud(2);
    println!("map {:?}", some.map(move |x| x + a.get()));
    let a = Loud(3);
    println!("and_then {:?}", none.and_then(move |x| Some(x + a.get())));
    let a = Loud(4);
    println!("filter {:?}", none.filter(move |x| *x > a.get()));
    let a = Loud(5);
    println!("is_some_and {}", none.is_some_and(move |x| x > a.get()));
    let a = Loud(6);
    println!("is_none_or {}", none.is_none_or(move |x| x > a.get()));
    let a = Loud(7);
    println!("map_or {}", none.map_or(0, move |x| x + a.get()));
    let (a, b) = (Loud(8), Loud(9));
    println!("map_or_else {}", some.map_or_else(move || a.get(), move |x| x + b.get()));
    let a = Loud(10);
    println!("unwrap_or_else {}", some.unwrap_or_else(move || a.get()));
    let a = Loud(24);
    println!("ok_or_else {:?}", some.ok_or_else(move || a.get()));
    let a = Loud(25);
    println!("or_else {:?}", some.or_else(move || Some(a.get())));
    let a = Loud(26);
    println!("then {:?}", false.then(move || a.get()));
    let a = Loud(27);
    println!("then {:?}", true.then(move || a.get()));
    let ok: Result<u32, u32> = Ok(1);
    let err: Result<u32, u32> = Err(2);
    let a = Loud(11);
    println!("map {:?}", err.map(move |x| x + a.get()));
    let a = Loud(12);
    println!("map_err {:?}", ok.map_err(move |e| e + a.get()));
    let a = Loud(13);
    println!("and_then {:?}", err.and_then(move |x| Ok(x + a.get())));
    let a = Loud(14);
    println!("unwrap_or_else {}", ok.unwrap_or_else(move |e| e + a.get()));
    let a = Loud(15);
    println!("map_or {}", err.map_or(0, move |x| x + a.get()));
    let (a, b) = (Loud(16), Loud(17));
    println!("map_or_else {}", ok.map_or_else(move |e| e + a.get(), move |x| x + b.get()));
    let a = Loud(18);
    println!("is_ok_and {}", err.is_ok_and(move |x| x > a.get()));
    let a = Loud(19);
    println!("is_err_and {}", ok.is_err_and(move |e| e > a.get()));
    let a = Loud(28);
    println!("or_else {:?}", ok.or_else(move |e| Err::<u32, u32>(e + a.get())));
    let a = Loud(20);
    println!("generic {}", or_else(some, move || a.get()));
    let a = Loud(21);
    println!("generic {}", or_else(none, move || a.get()));
    let mut counts: HashMap<&str, u32> = HashMap::new();
    counts.insert("x", 1);
    let a = Loud(22);
    *counts.entry("x").or_insert_with(move || a.get()) += 1;
    let mut slot = Some(3);
    let a = Loud(23);
    println!("get_or_insert_with {}", slot.get_or_insert_with(move || a.get()));
    println!("end {}", counts["x"]);
}
