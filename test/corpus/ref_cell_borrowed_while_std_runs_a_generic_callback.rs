use std::cell::RefCell;

fn add(log: &RefCell<Vec<i32>>, f: impl Fn(i32) -> bool) {
    let mut guard = log.borrow_mut();
    let free = Some(1).map(&f).unwrap_or(false);
    guard.push(i32::from(free));
}

fn main() {
    let log = RefCell::new(vec![0]);
    add(&log, |_| log.try_borrow().is_ok());
    println!("{:?}", log.borrow());
}
