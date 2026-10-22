// A `&mut self` method of a type parameter, called through a `RefCell`'s
// guard: the cell's value given in a box and read back, as a `&mut` of a type
// parameter is, of an object and of a number alike, read once.
use std::cell::RefCell;

trait Account {
    fn deposit(&mut self, n: u32);
}

struct Acc {
    b: u32,
}

impl Account for Acc {
    fn deposit(&mut self, n: u32) {
        self.b += n
    }
}

impl Account for u32 {
    fn deposit(&mut self, n: u32) {
        *self += n
    }
}

fn pay<A: Account>(c: &RefCell<A>) {
    c.borrow_mut().deposit(5)
}

fn pay_through<A: Account>(c: &RefCell<A>) {
    let mut guard = c.borrow_mut();
    let account: &mut A = &mut guard;
    account.deposit(7);
}

fn give<A: Account>(a: &mut A) {
    a.deposit(1);
}

fn pay_given<A: Account>(c: &RefCell<A>) {
    give(&mut *c.borrow_mut());
}

fn main() {
    let h = RefCell::new(Acc { b: 0 });
    pay(&h);
    pay_through(&h);
    pay_given(&h);
    println!("{}", h.borrow().b);
    let n = RefCell::new(1u32);
    pay(&n);
    pay_through(&n);
    pay_given(&n);
    println!("{}", n.borrow());
}
