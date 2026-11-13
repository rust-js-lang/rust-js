// A `RefCell` its destructors borrow, of values `truncate`, `retain` and a
// map's `clear` drop through a reference: each std call may drop a value of
// its type arguments, so `try_borrow_mut` is `false` while `seen` holds the
// cell (#1661).
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::rc::Rc;

struct Guard(Rc<RefCell<Vec<i32>>>);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("free: {}", self.0.try_borrow_mut().is_ok());
    }
}

fn keep_first(g: &RefCell<Vec<Guard>>) {
    g.borrow_mut().truncate(1)
}

fn keep_none(g: &RefCell<Vec<Guard>>) {
    g.borrow_mut().retain(|_| false)
}

fn clear_map(g: &RefCell<BTreeMap<u8, Guard>>) {
    g.borrow_mut().clear()
}

fn main() {
    let log = Rc::new(RefCell::new(vec![0]));
    let guards = RefCell::new(vec![Guard(Rc::clone(&log)), Guard(Rc::clone(&log))]);
    let map = RefCell::new(BTreeMap::from([(1, Guard(Rc::clone(&log)))]));
    let n = {
        let seen = log.borrow();
        keep_first(&guards);
        keep_none(&guards);
        clear_map(&map);
        seen.len()
    };
    println!("{n}");
}
