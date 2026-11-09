// `Rc::as_ptr` of two `Rc<dyn Fn>`s of one function, cast to a thin
// pointer and compared: two allocations, apart to Rust, so each is a
// function of its own in its counted `Rc`'s cell, `{ value, strong, weak
// }`: a function item's, `(arg0) => double(arg0)`, where it's unsized, and
// a cloned closure's that drops what it holds where it's given `Rc::new`,
// carrying its drop (#1053).
use std::rc::Rc;

struct Loud(&'static str);

impl Clone for Loud {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Loud(self.0)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    let first: Rc<dyn Fn(u32) -> u32> = Rc::new(double);
    let second: Rc<dyn Fn(u32) -> u32> = Rc::new(double);
    let again = Rc::clone(&first);
    let a = Rc::as_ptr(&first) as *const ();
    let b = Rc::as_ptr(&second) as *const ();
    let c = Rc::as_ptr(&again) as *const ();
    println!("item {} {} {}", a == b, a == c, first(1) + second(2) + again(3));
    // One unsized where it's held, not where it's made.
    let held = Rc::new(double);
    let third: Rc<dyn Fn(u32) -> u32> = held;
    let d = Rc::as_ptr(&third) as *const ();
    println!("held {} {}", a == d, third(4));

    let loud = Loud("held");
    let read = move || {
        let held = &loud;
        held.0.len()
    };
    let one: Rc<dyn Fn() -> usize> = Rc::new(read.clone());
    let two: Rc<dyn Fn() -> usize> = Rc::new(read);
    let one_at = Rc::as_ptr(&one) as *const ();
    let two_at = Rc::as_ptr(&two) as *const ();
    println!("cloned {} {}", one_at == two_at, one() + two());
    drop(one);
    println!("one gone");
}
