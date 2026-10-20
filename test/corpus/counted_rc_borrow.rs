//@ library-refused: another crate may share
// `rc.borrow()` of a counted `Rc` is what it points at, as `*rc` is: its
// `value`, not the `Rc` (ADR 0503); and one of a reference is shown as what
// that refers to (ADR 0504).
use std::borrow::Borrow;
use std::rc::Rc;
fn main() {
    let shared = Rc::new(String::from("rc"));
    let other = Rc::clone(&shared);
    let text: &String = shared.borrow();
    println!("{} {} {}", text, other, Rc::strong_count(&shared));
    let n = 5;
    let c: Rc<&i32> = Rc::new(&n);
    let d = Rc::clone(&c);
    println!("{c} {d:?} {}", Rc::strong_count(&c));
}
