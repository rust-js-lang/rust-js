//! A consumer giving the library's generic code values whose destructors
//! ask of the cell its guard is held on (#1325).
use std::cell::RefCell;
use std::rc::Rc;

use dep::Pushed;

struct Plain(Rc<RefCell<Vec<u8>>>);

impl Drop for Plain {
    fn drop(&mut self) {
        println!("plain: free {}", self.0.try_borrow().is_ok());
    }
}

struct Captured(Rc<RefCell<Vec<i32>>>);

impl Drop for Captured {
    fn drop(&mut self) {
        println!("captured: free {}", self.0.try_borrow().is_ok());
    }
}

struct Own(Rc<RefCell<Vec<u16>>>);

impl Drop for Own {
    fn drop(&mut self) {
        println!("own: free {}", self.0.try_borrow().is_ok());
    }
}

impl Pushed for Own {}

pub fn main() {
    let plain = Rc::new(RefCell::new(Vec::new()));
    dep::push_after(Plain(plain.clone()), &plain);
    let captured = Rc::new(RefCell::new(Vec::new()));
    dep::push_in_closure(Captured(captured.clone()), &captured);
    let own = Rc::new(RefCell::new(Vec::new()));
    Own(own.clone()).push_self(&own);
    println!("{:?} {:?} {:?}", plain.borrow(), captured.borrow(), own.borrow());
}
