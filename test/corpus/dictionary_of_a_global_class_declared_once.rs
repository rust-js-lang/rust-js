//@ library-differs
// A dictionary whose entry reads a JS global, the `Map` an impl's map
// class of its own parameter is, `bagTally(Map)`, is made once at the top
// of the body that gives it twice, as one that reads only helpers is
// (ADR 0979, #1239).
use std::collections::HashMap;
use std::hash::Hash;

trait Tally {
    fn tally(&self) -> usize;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Color {
    Red,
    Green,
    Blue,
}

struct Bag<K> {
    items: Vec<K>,
}

impl<K: Hash + Eq + Clone> Tally for Bag<K> {
    fn tally(&self) -> usize {
        let mut counts: HashMap<K, usize> = HashMap::new();
        for item in &self.items {
            *counts.entry(item.clone()).or_insert(0) += 1;
        }
        counts.len()
    }
}

fn distinct<T: Tally>(of: &T) -> usize {
    of.tally()
}

fn main() {
    let words = Bag {
        items: vec![Color::Red, Color::Green, Color::Red],
    };
    let more = Bag {
        items: vec![Color::Blue, Color::Blue],
    };
    println!("{} {}", distinct(&words), distinct(&more));
}
