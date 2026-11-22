//! A consumer giving a library's generic functions its `Pool` of items
//! with a destructor, which it compiles itself, of a `Vec`'s items, which
//! are `TrustedLen`, and of its own iterator, which isn't (#2327).
use dep::{Pool, fold_pool, last_of_pool};

#[derive(Debug)]
struct Word(u32);

impl Drop for Word {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Words(u32);

impl Iterator for Words {
    type Item = Word;

    fn next(&mut self) -> Option<Word> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some(Word(self.0))
    }
}

fn pool<I: Iterator<Item = Word>>(it: I) -> Pool<I> {
    Pool { it, items: Vec::new() }
}

pub fn main() {
    println!("{:?}", last_of_pool(pool(Words(3))));
    println!("{:?}", last_of_pool(pool(vec![Word(8), Word(9)].into_iter())));
    println!("{}", fold_pool(pool(Words(2))));
    println!("{}", fold_pool(pool(vec![Word(7)].into_iter())));
}
