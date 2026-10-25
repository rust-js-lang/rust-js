// A map's `len()`, read where it's used in a loop's branch, isn't read again
// before a drop after the loop, where the map isn't (a generated program's,
// seed 12446640; ADR 0451).

use std::collections::BTreeMap;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn id<T>(x: T) -> T {
    x
}

// Dropped after the loop only.
fn dropped_after() {
    let loud = Loud(5);
    let mut w = 0u32;
    while w < id(2u32) {
        w += 1;
        let size = match (BTreeMap::<u8, i32>::new().len() as u32, id(1u16)) {
            (7, _) => 1,
            (_, y) if id(false) => y,
            _ => 2,
        };
        println!("{size}");
    }
    println!("{}", loud.0);
}

// Dropped at the end of each pass too, where the next one's `len()` is a
// new value, not one to read before it.
fn dropped_each_pass() {
    let mut w = 0u32;
    while w < id(2u32) {
        w += 1;
        let _each = Loud(w + 10);
        let size = match (BTreeMap::<u8, i32>::new().len() as u32, id(1u16)) {
            (7, _) => 1,
            (_, y) if id(false) => y,
            _ => 2,
        };
        println!("{size}");
    }
}

// Read in the branch that uses it, then tested past where the branches
// meet, before a drop: not read again there, as the playground's `stem`.
fn out_file(root: &str, test: bool) -> String {
    let loud = Loud(7);
    let out = match root.strip_suffix(".rs") {
        Some(stem) => format!("/out/{stem}.js"),
        None => format!("/out/{root}"),
    };
    let mut args = vec![String::from("rust-js")];
    if test {
        args.push(String::from("--test"));
    }
    args.push(out);
    println!("{}", loud.0);
    args.join(" ")
}

fn main() {
    dropped_after();
    dropped_each_pass();
    println!("{}", out_file("main.rs", true));
    println!("{}", out_file("lib", false));
}
