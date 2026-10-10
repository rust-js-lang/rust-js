//@ run-fail: clone failed
// A `make_mut` whose clone panics leaves the `Rc` shared, its count as it
// was, as std's does: what drops as the panic unwinds sees every owner.

use std::rc::Rc;

struct Fails;

impl Clone for Fails {
    fn clone(&self) -> Self {
        panic!("clone failed")
    }
}

struct Watch(Rc<Fails>);

impl Drop for Watch {
    fn drop(&mut self) {
        println!("owners as it unwinds: {}", Rc::strong_count(&self.0));
    }
}

fn main() {
    let mut first = Rc::new(Fails);
    let _watch = Watch(Rc::clone(&first));
    Rc::make_mut(&mut first);
}
