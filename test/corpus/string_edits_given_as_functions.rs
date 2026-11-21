// A `String` method that changes the string, given as a function value,
// does what its closure does: `String::clear` given to a thread-local's
// `with_borrow_mut`, to an `Option`'s `map` and to `for_each`, and
// `String::clear` and `String::push_str` as `fn` pointers. Each gets the
// `&mut String` as a handle, whose `value` it writes (#946).
use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<String> = RefCell::new(String::new());
}

fn apply(f: fn(&mut String, &str), s: &mut String, more: &str) {
    f(s, more);
}

fn main() {
    LOG.with_borrow_mut(|log| log.push_str("abc"));
    LOG.with_borrow(|log| println!("before [{log}]"));
    LOG.with_borrow_mut(String::clear);
    LOG.with_borrow(|log| println!("after [{log}]"));

    let mut t = String::from("xy");
    let cleared = Some(&mut t).map(String::clear);
    println!("{cleared:?} [{t}]");

    let f: fn(&mut String) = String::clear;
    let mut u = String::from("uv");
    f(&mut u);
    println!("[{u}]");

    let mut words = vec![String::from("a"), String::from("b")];
    words.iter_mut().for_each(String::clear);
    println!("{words:?}");

    let mut w = String::from("w");
    apply(String::push_str, &mut w, "z");
    apply(String::push_str, &mut w, "!");
    println!("[{w}]");
}
