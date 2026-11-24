// A borrow held while a closure runs that asks `try_borrow_mut()` of its
// cell is counted, so the closure finds it borrowed, as Rust's does: the
// closure drops the `Result` it made, which the analysis took for code that
// may run anything, and lost the closure's own borrow (#2450, #2451). Of a
// closure `Option::map` runs, one an iterator's `map` runs, and a `Drop`
// that asks while a borrow is held.
use std::cell::RefCell;

struct Asks<'a>(&'a RefCell<Vec<u8>>);

impl Drop for Asks<'_> {
    fn drop(&mut self) {
        println!("busy while dropped: {}", self.0.try_borrow_mut().is_err());
    }
}

fn main() {
    let cell: RefCell<Option<u32>> = RefCell::new(Some(4));
    let t = cell.borrow().map(|v| (v, cell.try_borrow_mut().is_ok()));
    println!("{t:?}");
    println!("free after: {}", cell.try_borrow_mut().is_ok());

    let items = RefCell::new(vec![1u8, 2]);
    let busy: Vec<bool> = items.borrow().iter().map(|_| items.try_borrow_mut().is_err()).collect();
    println!("{busy:?}");

    let held = RefCell::new(vec![3u8]);
    {
        let guard = held.borrow();
        let asks = Asks(&held);
        drop(asks);
        println!("{}", guard.len());
    }
    drop(Asks(&held));
}
