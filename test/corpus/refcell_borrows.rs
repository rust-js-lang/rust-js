// `RefCell`'s borrows, counted as std's are (ADR 0328): shared ones side by
// side, one released at its scope's end or its `drop`, and `try_borrow`.

use std::cell::RefCell;
use std::rc::Rc;

struct Counter {
    hits: RefCell<u32>,
}

impl Counter {
    fn hit(&self) -> u32 {
        *self.hits.borrow_mut() += 1;
        *self.hits.borrow()
    }
}

fn main() {
    let cell = RefCell::new(vec![1, 2]);
    {
        let a = cell.borrow();
        let b = cell.borrow();
        println!("{} {} {}", a.len(), b[0], cell.try_borrow_mut().is_err());
    }
    cell.borrow_mut().push(3);
    let writing = cell.borrow_mut();
    println!("{} {}", cell.try_borrow().is_err(), writing.len());
    drop(writing);
    println!("{:?} {}", cell.try_borrow().map(|v| v.len()), cell.try_borrow_mut().is_ok());
    *cell.borrow_mut() = vec![7];
    let number = RefCell::new(1);
    *number.borrow_mut() = 5;
    let text = RefCell::new(String::new());
    *text.borrow_mut() = String::from("t");
    println!("{:?} {} {}", cell.borrow(), number.borrow(), text.borrow());
    let counter = Counter { hits: RefCell::new(0) };
    counter.hit();
    println!("{}", counter.hit());
    let shared = Rc::new(RefCell::new(String::from("a")));
    let other = Rc::clone(&shared);
    other.borrow_mut().push('b');
    for _ in 0..2 {
        shared.borrow_mut().push('c');
    }
    println!("{} {}", shared.borrow(), other.borrow().len());
    let held = shared.borrow_mut();
    println!("{:?} {:?}", shared, held);
    drop(held);
    let copy = RefCell::clone(&shared);
    println!("{}", copy == *shared);
    println!("{:?} {} {}", shared.replace(String::from("z")), copy == *shared, shared.take());
    let reading = copy.borrow();
    println!("{:?} {:?} {}", copy, reading, copy.try_borrow_mut().is_err());
}
