//@ compile-fail: `std::rc::Rc::<(dyn std::any::Any + 'static), A>::downcast` of a value with a destructor
// A counted `Rc` holds its `dyn Any`'s pair in its `value`, which a
// downcast takes by value, as one of a destructor: refused (ADR 0331).

use std::any::Any;
use std::rc::Rc;

fn main() {
    let shared: Rc<dyn Any> = Rc::new(5u8);
    println!("{}", Rc::strong_count(&shared));
    println!("{:?}", shared.downcast::<u8>().is_ok());
}
