// A count read of a concrete `Rc` in a crate with no destructor, where
// generic code clones an `Rc` of its type parameter: every `Rc` is counted,
// as the generic one may be the counted one (ADRs 0320, 0494).
use std::rc::Rc;

fn again<T>(rc: &Rc<T>) -> Rc<T> {
    rc.clone()
}

fn main() {
    let rc = Rc::new(5u8);
    let other = again(&rc);
    println!("{} {}", Rc::strong_count(&rc), other);
    drop(other);
    println!("{}", Rc::strong_count(&rc));
}
