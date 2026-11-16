// A local named as one of its module's names: an imported function's,
// `parse`, read before the local takes its name and called again in
// another function, and the module's own `cmp`, beside the `$cmp` a
// comparison of strings reads. Each local is the function's, apart from
// the module's name of its word ("names are ids", ADR 1150).
mod text {
    pub fn parse(word: &str) -> i32 {
        word.len() as i32
    }
}

use std::cmp::Ordering;
use text::parse;

fn cmp(a: i32, b: i32) -> Ordering {
    b.cmp(&a)
}

pub fn ranked(a: &str, b: &str) -> i32 {
    let parse = parse(a) + parse(b);
    let cmp = a.cmp(b);
    match cmp {
        Ordering::Less => parse,
        Ordering::Equal => 0,
        Ordering::Greater => -parse,
    }
}

pub fn reversed(a: i32, b: i32) -> bool {
    let cmp = cmp(a, b);
    cmp == Ordering::Less
}

fn main() {
    println!("{}", ranked("apple", "pear"));
    println!("{}", ranked("pear", "apple"));
    println!("{}", ranked("fig", "fig"));
    println!("{} {}", reversed(1, 2), reversed(2, 1));
    println!("{}", parse("again"));
}
