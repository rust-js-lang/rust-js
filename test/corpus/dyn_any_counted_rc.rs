//@ library-refused: another crate may share
// A counted `Rc<dyn Any>` is its pair, `{ value, impl, strong, weak }`, so
// a downcast is the same `Rc`, its counts shared with each clone left of
// the `dyn`; one that fails gives it back (ADR 0520).
use std::any::Any;
use std::rc::Rc;

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let shared: Rc<dyn Any> = Rc::new(5u8);
    println!("{}", Rc::strong_count(&shared));
    println!("{:?}", shared.downcast::<u8>().is_ok());
    let loud: Rc<dyn Any> = Rc::new(Loud(1));
    let other = Rc::clone(&loud);
    let back = match loud.downcast::<u8>() {
        Ok(_) => unreachable!(),
        Err(back) => back,
    };
    let found = back.downcast::<Loud>().ok().unwrap();
    println!("{} {}", found.0, Rc::strong_count(&found));
    drop(found);
    println!("{}", Rc::strong_count(&other));
    drop(other);
    println!("end");
}
