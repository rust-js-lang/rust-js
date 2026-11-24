//@ compile-fail: a `RefCell` guard's `&mut`, `&mut std::cell::RefMut<'_, std::vec::Vec<i32>>`, given to generic code's `&mut T`, which may write the guard whole
// A `RefCell`'s guard given by `&mut` to generic code whose `&mut T` it may
// write whole, `*slot = v`: Rust's `put` drops `x`'s guard and makes the
// variable `y`'s, which the block's end gives back. The JS wrote `y`'s
// guard into `x`'s object, in place, and never gave `y`'s borrow back, so
// it's refused until a form that rebinds the variable exists (#2475).
use std::cell::RefCell;

fn put<T>(slot: &mut T, v: T) {
    *slot = v;
}

fn main() {
    let x = RefCell::new(vec![1]);
    let y = RefCell::new(vec![2]);
    {
        let mut g = x.borrow_mut();
        let h = y.borrow_mut();
        put(&mut g, h);
        g.push(5);
        println!("x free {}, y free {}", x.try_borrow_mut().is_ok(), y.try_borrow_mut().is_ok());
    }
    println!("{:?} {:?}", x.try_borrow().map(|v| v.clone()), y.try_borrow().map(|v| v.clone()));
}
