//@ run-abort
// A bound's check that fails panics, and what's held is dropped as it
// unwinds: an `Rc`'s last, whose value's drop panics, aborts (#847), as
// Rust does. One whose drop doesn't panic is dropped on the way.
use std::panic;
use std::rc::Rc;

struct Bomb(&'static str);

impl Drop for Bomb {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == "bomb" {
            panic!("bomb went off");
        }
    }
}

fn at(items: &[u32], i: usize, held: Rc<Bomb>) -> u32 {
    let _held = held;
    items[i]
}

fn main() {
    let r = panic::catch_unwind(|| at(&[1, 2], 5, Rc::new(Bomb("calm"))));
    println!("caught {}", r.is_err());
    let shared = Rc::new(Bomb("bomb"));
    let r = panic::catch_unwind(move || at(&[1, 2], 7, shared));
    println!("never {}", r.is_err());
}
