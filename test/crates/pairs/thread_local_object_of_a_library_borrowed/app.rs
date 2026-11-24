//! A library's thread-local whose borrows nothing counts, borrowed here, is
//! given as the library's variable, `log.push(1)`, as its own module's
//! borrows are: counted, it read the `value` of the array and threw where
//! Rust prints (#2531).

pub fn main() {
    dep::LOG.with_borrow_mut(|log| log.push(1));
    dep::LOG.with_borrow_mut(|log| log.extend([2, 3]));
    println!("{:?} {}", dep::LOG.with_borrow(|log| log.clone()), dep::logged());
}
