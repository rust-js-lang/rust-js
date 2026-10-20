//@ library-refused: another crate may share
// `rc.borrow()` of a counted `Rc` is what it points at, as `*rc` is: its
// `value`, not the `Rc` (ADR 0503).
use std::borrow::Borrow;
use std::rc::Rc;
fn main() {
    let shared = Rc::new(String::from("rc"));
    let other = Rc::clone(&shared);
    let text: &String = shared.borrow();
    println!("{} {} {}", text, other, Rc::strong_count(&shared));
}
