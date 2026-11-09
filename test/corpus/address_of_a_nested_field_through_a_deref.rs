//@ run-pass
// A raw pointer to a nested place whose root is what a `Deref` gives, a
// `RefCell`'s `Ref` and `RefMut`, an `Rc`'s, a `Box`'s or a reference a
// function returns, is at its offset in that root's allocation, as
// `&raw const items[1].b` is (#1089). Its root was written as the MIR
// temporary that held it, a name no JS declares, `_25`, which threw
// (#1110). An item of an array a tuple holds, `(wrapped.0)[1]`, whose own
// place says no offset, is rooted too (#1110).
use std::cell::RefCell;
use std::rc::Rc;

struct P {
    a: u8,
    b: u32,
}

fn ps() -> [P; 3] {
    [P { a: 1, b: 10 }, P { a: 2, b: 20 }, P { a: 3, b: 30 }]
}

fn first<'a>(items: &'a [P; 3]) -> &'a [P; 3] {
    items
}

fn main() {
    let cell = RefCell::new(ps());
    let base = &raw const *cell.borrow() as usize;
    println!("ref {}", &raw const (*cell.borrow())[2].a as usize - base);
    let mut seen = Vec::new();
    for i in 0..cell.borrow().len() {
        seen.push(&raw const (*cell.borrow())[i].b as usize - base);
    }
    println!("ref {seen:?}");
    let other = RefCell::new(ps());
    {
        let mut held = other.borrow_mut();
        let base = &raw mut *held as usize;
        println!("ref mut {}", &raw mut (*held)[1].b as usize - base);
    }

    let rc = Rc::new(ps());
    let base = &raw const *rc as usize;
    println!("rc {}", &raw const (*rc)[1].a as usize - base);

    let boxed = Box::new(ps());
    let base = &raw const *boxed as usize;
    println!("box {}", &raw const (*boxed)[2].b as usize - base);

    let items = ps();
    let base = &raw const items as usize;
    println!("returned {}", &raw const (*first(&items))[1].b as usize - base);
    println!("{} {}", items[1].a, items[2].b);

    // An item of an array a tuple holds, whose own place says no offset,
    // its root's does: the same allocation as its field's.
    let wrapped = ([(0u8, 0u32), (1, 1), (2, 2)],);
    let second = &raw const (wrapped.0)[1];
    let start = &raw const (wrapped.0)[0] as usize;
    println!(
        "wrapped {} {}",
        &raw const (wrapped.0)[1].1 as usize - start,
        &raw const (wrapped.0)[1].0 as usize - second as usize
    );
}
