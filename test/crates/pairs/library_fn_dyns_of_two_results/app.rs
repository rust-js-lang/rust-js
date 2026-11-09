//! The reads compared, counted here; the listener, of another result,
//! called as the library hands it (#1056).
use dep::{listener, reads};
use std::rc::Rc;

pub fn main() {
    let (first, second) = reads(7);
    println!("{} {} {}", first(), second(), Rc::ptr_eq(&first, &second));
    let heard = listener("ping");
    let again = Rc::clone(&heard);
    heard();
    again();
}
