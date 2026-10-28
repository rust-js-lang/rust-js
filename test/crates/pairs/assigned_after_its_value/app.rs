use std::cell::RefCell;
use std::rc::Rc;

fn tracker(current: Rc<RefCell<String>>) -> impl Fn(&str) {
    // Rust evaluates the value, `shout`'s call, before the place it's
    // written to: no borrow is held while the library's code runs.
    move |value: &str| {
        *current.borrow_mut() = dep::shout(value);
    }
}

pub fn main() {
    let current = Rc::new(RefCell::new(String::new()));
    let track = tracker(Rc::clone(&current));
    track("ada");
    println!("{}", current.borrow());
    dep::set(&current, "grace");
    track("hopper");
    println!("{}", current.borrow());
}
