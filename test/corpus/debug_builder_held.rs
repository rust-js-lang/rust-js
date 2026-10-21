// A `Debug` builder held in a variable, stepped in a branch or a loop, or
// not at all: each step's part as it's taken, and `finish` or
// `finish_non_exhaustive` of what it was given, as Rust's writes it, plain
// or pretty (ADR 0532).
use std::fmt;

struct Pair {
    first: Option<u32>,
    second: Option<&'static str>,
}

impl fmt::Debug for Pair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dbg = f.debug_struct("Pair");
        if let Some(first) = self.first {
            dbg.field("first", &first);
        }
        match self.second {
            Some(second) => {
                dbg.field("second", &second);
            }
            None => {}
        }
        dbg.finish()
    }
}

struct Evens(Vec<u32>);

impl fmt::Debug for Evens {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        for x in &self.0 {
            if x % 2 == 0 {
                list.entry(x);
            }
        }
        list.finish()
    }
}

struct Open(Vec<u32>);

impl fmt::Debug for Open {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dbg = f.debug_struct("Open");
        if let Some(first) = self.0.first() {
            dbg.field("first", first);
        }
        dbg.finish_non_exhaustive()
    }
}

struct Row(Vec<u32>);

impl fmt::Debug for Row {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut row = f.debug_tuple("Row");
        for x in &self.0 {
            row.field(x);
        }
        row.finish()
    }
}

struct Tags(Vec<&'static str>);

impl fmt::Debug for Tags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut set = f.debug_set();
        set.entries(&self.0);
        if self.0.is_empty() {
            set.entry(&"none");
        }
        set.finish()
    }
}

fn main() {
    let both = Pair { first: Some(1), second: Some("two") };
    let one = Pair { first: None, second: Some("only") };
    let none = Pair { first: None, second: None };
    println!("{both:?} {one:?} {none:?}");
    println!("{both:#?}");
    println!("{:?} {:?}", Evens(vec![1, 2, 3, 4]), Evens(vec![1]));
    println!("{:#?} {:#?}", Evens(vec![2]), Evens(vec![]));
    println!("{:?} {:?} {:#?} {:#?}", Open(vec![]), Open(vec![7]), Open(vec![]), Open(vec![7]));
    println!("{:?} {:?} {:#?}", Row(vec![]), Row(vec![1, 2]), Row(vec![3]));
    println!("{:?} {:?} {:#?}", Tags(vec![]), Tags(vec!["a", "b"]), Tags(vec!["c"]));
}
