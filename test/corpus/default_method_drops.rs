// A trait's default method, copied into each impl (ADR 0049), is checked
// for what it drops as any body is (ADR 0098): a temporary with a
// destructor, not dropped, would print one line fewer than Rust.
use std::fmt::Debug;

#[derive(Debug)]
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// A list inside itself drops by a function of its own, declared once, at
// the top of the copy.
struct List {
    item: Noisy,
    next: Option<Box<List>>,
}

trait Greet {
    fn id(&self) -> u32;

    fn greet(&self) {
        if let Some(_) = &Some(Noisy(1)) {
            println!("hello");
        }
    }

    // A temporary taken apart, and a `&dyn` and a slice of values with a
    // destructor, which own nothing to drop: each is supported.
    fn parts(&self) -> usize {
        let (first, _) = (Noisy(2), Noisy(3));
        let shown: &dyn Debug = &first;
        let all = [Noisy(4), Noisy(5)];
        let slice: &[Noisy] = &all;
        format!("{shown:?}").len() + slice.len()
    }

    fn lists(&self, long: bool) -> u32 {
        let first = List { item: Noisy(6), next: None };
        if long {
            let second = List { item: Noisy(7), next: Some(Box::new(List { item: Noisy(8), next: None })) };
            second.item.0
        } else {
            first.item.0
        }
    }
}

struct Person;

impl Greet for Person {
    fn id(&self) -> u32 {
        7
    }
}

fn main() {
    Person.greet();
    println!("{}", Person.parts());
    println!("{} {}", Person.lists(true), Person.lists(false));
}
