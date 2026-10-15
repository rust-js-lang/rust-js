// A `Box<RefCell<dyn T>>` dropped drops what it holds through its cell, the
// pair it is: `cell.impl.$drop(cell.value)` (ADR 0397).
use std::cell::RefCell;

trait Named {
    fn name(&self) -> String;
    fn rename(&mut self, to: &str);
}

struct Noisy(String);

impl Named for Noisy {
    fn name(&self) -> String {
        self.0.clone()
    }
    fn rename(&mut self, to: &str) {
        self.0 = to.to_string();
    }
}

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let named: Box<RefCell<dyn Named>> = Box::new(RefCell::new(Noisy("first".to_string())));
    named.borrow_mut().rename("renamed");
    println!("{}", named.borrow().name());
    {
        let inner: Box<RefCell<dyn Named>> = Box::new(RefCell::new(Noisy("inner".to_string())));
        println!("{}", inner.borrow().name());
    }
    println!("end");
}
