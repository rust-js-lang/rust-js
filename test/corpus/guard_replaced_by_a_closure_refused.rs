//@ compile-fail: a `RefCell` guard written whole by a closure that borrows its variable
// A guard a closure takes by reference and writes whole, `|| state =
// other`, is refused (#2475): the variable's release is decided by the
// guard its `let` gives it, uncounted where nothing can ask, so another
// cell's counted guard written in the closure was never released, and the
// next borrow of that cell panicked "already mutably borrowed" where Rust
// prints `Pending { tries: 0 } Pending { tries: 9 }`. Written where the
// variable is, `state = other;`, it compiles and runs as Rust's does.
use std::cell::{RefCell, RefMut};

#[derive(Debug)]
enum Status {
    Pending { tries: u32 },
    Done { value: String },
}

fn kept<'a>(g: &'a mut RefMut<'_, Status>) -> &'a mut Status {
    &mut **g
}

fn main() {
    let a = RefCell::new(Status::Pending { tries: 0 });
    let plain = RefCell::new(Status::Done { value: "p".into() });
    {
        let mut state = a.borrow_mut();
        let other = plain.borrow_mut();
        let swap_in = || state = other;
        swap_in();
        *kept(&mut state) = Status::Pending { tries: 9 };
    }
    println!("{:?} {:?}", a.borrow(), plain.borrow());
    if let Status::Pending { tries } = &*a.borrow() {
        println!("{tries}");
    }
    if let Status::Done { value } = &*plain.borrow() {
        println!("{value}");
    }
}
