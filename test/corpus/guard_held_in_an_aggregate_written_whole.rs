// A cell's `RefMut` kept in a tuple's, a struct's or an array's place, or
// one nested in another, written whole through, `*(items.0) = v`: what its
// `deref_mut` gives is the cell's value, the cell's own object in JS, so
// the write replaces that object's contents, `$assign`, as a guard a
// variable holds is written; not the place, which left the cell its old
// value and borrowed (#1841).
use std::cell::{RefCell, RefMut};

struct Holder<'a> {
    guard: RefMut<'a, Vec<u32>>,
}

fn main() {
    let queue = RefCell::new(vec![1, 2]);
    {
        let mut items = (queue.borrow_mut(),);
        *(items.0) = vec![7];
    }
    println!("tuple {:?}", queue.borrow());
    queue.borrow_mut().push(8);
    println!("then {:?}", queue.borrow());
    {
        let mut h = Holder { guard: queue.borrow_mut() };
        *h.guard = vec![9];
    }
    println!("struct {:?}", queue.borrow());
    {
        let mut a = [queue.borrow_mut()];
        *a[0] = vec![10];
    }
    println!("array {:?}", queue.borrow());
    {
        let mut nested = ((queue.borrow_mut(), 1u8),);
        *(nested.0).0 = vec![11];
        nested.0.0.push(12);
    }
    println!("nested {:?}", queue.borrow());
    // A cell of what isn't an object, `{ value }`: its guard gives the
    // cell's `value`, the place written.
    let count = RefCell::new(5u32);
    {
        let mut kept = (count.borrow_mut(), 0u8);
        *kept.0 = 6;
        *kept.0 += 1;
    }
    println!("count {}", count.borrow());
    let text = RefCell::new(String::from("a"));
    {
        let mut kept = [text.borrow_mut()];
        *kept[0] = String::from("b");
        kept[0].push('c');
    }
    println!("text {}", text.borrow());
}
