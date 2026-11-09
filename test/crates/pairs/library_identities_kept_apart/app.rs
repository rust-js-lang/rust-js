//! Each identity as Rust keeps it (#1052).
use dep::{Pair, Twice, bumps, cloned, computed, double, doubled, pings, total};
use std::cell::RefCell;
use std::rc::Rc;

pub fn main() {
    println!("{}", total(Rc::new(vec![5, 6])));
    let count = Rc::new(RefCell::new(4));
    let (first, second) = cloned(&count);
    let again = Rc::clone(&first);
    println!("{} {} {} {}", first(), second(), Rc::ptr_eq(&first, &second), Rc::ptr_eq(&first, &again));
    let getter: fn(Option<&u32>) -> u32 = double;
    let a = computed(getter);
    let b = computed(getter);
    println!("same {}", std::ptr::addr_eq(&*a.compute, &*b.compute));
    let (c, d) = (doubled(), doubled());
    println!("same {}", std::ptr::addr_eq(&*c.compute, &*d.compute));
    let first = (a.compute)(None);
    println!("{first} {}", (b.compute)(Some(first.clone())));
    let (first, second) = bumps();
    println!("{} {} {}", first(2), second(3), std::ptr::addr_eq(&*first, &*second));
    let (first, second) = pings();
    first(true);
    println!("{}", Rc::ptr_eq(&first, &second));
    let (first, second) = Twice(5).pair();
    println!("{} {}", first(), Rc::ptr_eq(&first, &second));
    println!("{}", paired(&Twice(6)));
}

/// Through the impl's dictionary.
fn paired<P: Pair>(p: &P) -> bool {
    let (first, second) = p.pair();
    Rc::ptr_eq(&first, &second)
}
