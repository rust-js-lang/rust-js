// A `replace_with` holds its cell while its function runs, which borrows
// another cell of its type, made apart: by their flow two cells, so the
// function's borrow can't meet the held one, and neither is counted
// (rd482, #687).
use std::cell::RefCell;

fn main() {
    let a = RefCell::new(vec![1u32]);
    let b = RefCell::new(vec![10u32]);
    a.replace_with(|old| {
        let mut v = old.clone();
        v.push(b.borrow()[0]);
        v
    });
    b.borrow_mut().push(2);
    println!("{:?} {:?}", a.borrow(), b.borrow());
}
