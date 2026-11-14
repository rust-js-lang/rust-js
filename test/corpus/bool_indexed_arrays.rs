// An array indexed by a `bool` made an integer, `usize::from(apple)` or
// `apple as usize`, is 0 or 1 (#1863): of an array of two, in bounds, its
// index JS's own, `parsed[apple ? 1 : 0]`, as upstream's `parsed[+apple]`,
// a thread-local's array through its `with` too. Of an array of one, `true`
// is past it, the index checked, and panics as Rust's does.
use std::cell::RefCell;

thread_local! {
    static PARSED: [RefCell<Vec<u32>>; 2] = [RefCell::new(vec![1]), RefCell::new(vec![2, 3])];
}

fn parsed(apple: bool) -> usize {
    PARSED.with(|parsed| parsed[usize::from(apple)].borrow().len())
}

fn cast(apple: bool, pair: &[u32; 2]) -> u32 {
    pair[apple as usize]
}

fn single(apple: bool, one: &[u32; 1]) -> u32 {
    one[usize::from(apple)]
}

fn single_cast(apple: bool, one: &[u32; 1]) -> u32 {
    one[apple as usize]
}

fn main() {
    println!("{} {}", parsed(false), parsed(true));
    println!("{} {}", cast(false, &[4, 5]), cast(true, &[4, 5]));
    println!("{}", single(false, &[6]));
    let r = std::panic::catch_unwind(|| single(true, &[6]));
    println!("single: {}", if r.is_ok() { "ran" } else { "panicked" });
    let r = std::panic::catch_unwind(|| single_cast(true, &[6]));
    println!("single cast: {}", if r.is_ok() { "ran" } else { "panicked" });
}
