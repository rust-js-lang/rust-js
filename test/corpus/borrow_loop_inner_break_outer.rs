// A `for` over a borrow whose inner loop leaves the outer one, `break
// 'outer`, and goes on with it, `continue 'outer`, after a test that skips
// a turn: one `try { outer: for .. } finally { $unborrow(..) }`, whose
// loop ends where Rust's does (#441).

use std::cell::RefCell;

fn borrowed(items: &RefCell<Vec<u32>>, keep: &dyn Fn(u32) -> bool) -> u32 {
    let mut total = 0;
    'outer: for &x in items.borrow().iter() {
        if !keep(x) {
            continue;
        }
        let mut j = 0;
        while j < 3 {
            if j == x {
                continue 'outer;
            }
            if j + x > 8 {
                break 'outer;
            }
            total += j;
            j += 1;
        }
        total += 100;
    }
    total
}

fn main() {
    let items = RefCell::new(vec![1, 2, 5, 7]);
    println!("{}", borrowed(&items, &|x| x != 5));
    println!("{}", borrowed(&items, &|x| x > 4));
    println!("{}", borrowed(&items, &|x| items.borrow().len() > x as usize));
    items.borrow_mut().push(9);
    println!("{:?}", items.borrow());
}
