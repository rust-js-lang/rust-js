//! A crate that holds borrows of its own cells while its library's code
//! runs, and uses the library's cells it can reach and those it can't
//! (ADR 0863, amended, #456). Its held `RefCell<Vec<String>>` is no cell
//! the library borrows unchecked, so it isn't refused.
use std::cell::RefCell;

pub fn main() {
    let mine = RefCell::new(vec![1u32]);
    let held = mine.borrow();
    let names = RefCell::new(vec![String::from("a")]);
    let named = names.borrow();
    let made = dep::made();
    dep::push(&made);
    let open = dep::Open {
        items: RefCell::new(Vec::new()),
    };
    dep::fill(&open);
    let tally = dep::Tally::new();
    tally.add(3);
    tally.add(4);
    dep::log(6);
    let logged = dep::logged();
    dep::around(|n| print!("{n} {} ", named.len()));
    println!(
        "{} {:?} {:?} {} {} {} {logged}",
        held.len(),
        made.borrow(),
        open.items.borrow(),
        tally.total(),
        dep::labelled(),
        dep::stocked()
    );
}
