// A guard a `let` holds, while only code that can't borrow its cell runs
// until its block ends or it's dropped, meets no other borrow: its cell's
// borrows aren't counted, nor checked (ADR 0863). Of an array, a number, a
// struct, a field's, read through it and written through it; and one moved
// into `drop`.

use std::cell::RefCell;
use std::rc::Rc;

struct Totals {
    sum: i32,
    seen: Vec<i32>,
}

struct Holder {
    count: RefCell<u32>,
}

fn total(log: &Rc<RefCell<Vec<i32>>>) -> (i32, usize) {
    let v = log.borrow();
    let mut total = 0;
    for x in v.iter() {
        total += *x;
    }
    (total, v.len())
}

fn bump(holder: &Holder) -> u32 {
    let mut count = holder.count.borrow_mut();
    *count += 2;
    *count
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let writer = Rc::clone(&log);
    for i in 0..4 {
        writer.borrow_mut().push(i * i);
    }
    println!("{:?}", total(&log));

    let n = RefCell::new(1);
    {
        let mut v = n.borrow_mut();
        *v += 10;
        *v *= 2;
    }
    let read = n.borrow();
    println!("{}", *read + 1);
    drop(read);
    *n.borrow_mut() += 1;
    println!("{}", n.borrow());

    let totals = RefCell::new(Totals { sum: 0, seen: Vec::new() });
    for i in 1..=3 {
        let mut t = totals.borrow_mut();
        t.sum += i;
        t.seen.push(i);
    }
    let t = totals.borrow();
    println!("{} {:?}", t.sum, t.seen);
    drop(t);
    totals.borrow_mut().sum = 0;
    println!("{}", totals.borrow().sum);

    let holder = Holder { count: RefCell::new(0) };
    bump(&holder);
    println!("{} {}", bump(&holder), holder.count.borrow());
}
