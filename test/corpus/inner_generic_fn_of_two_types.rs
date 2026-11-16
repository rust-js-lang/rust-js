// A function written in a function's body, generic, called with two types
// in that body: one function, given each type's dictionary, its declaration
// and both its reads one name, the item's, made by two lowerings, its own
// and its body's ("names are ids", ADR 1150).
use std::fmt::Debug;

fn shown_twice(n: i32, word: &str) -> String {
    fn twice<T: Debug + Clone>(x: T) -> String {
        let pair = (x.clone(), x);
        format!("{pair:?}")
    }
    fn count() -> usize {
        3
    }
    format!("{} {} {}", twice(n), twice(word), count())
}

pub fn listed(items: &[u8]) -> Vec<String> {
    fn label<T: Debug>(at: usize, item: T) -> String {
        format!("{at}: {item:?}")
    }
    let mut out: Vec<String> = items.iter().enumerate().map(|(at, item)| label(at, item)).collect();
    out.push(label(items.len(), "end"));
    out
}

fn main() {
    println!("{}", shown_twice(7, "seven"));
    for line in listed(&[4, 2]) {
        println!("{line}");
    }
}
