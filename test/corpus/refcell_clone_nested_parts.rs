// A clone of a `RefCell` whose value's clone is made of parts, a tuple's
// in another `RefCell`, borrowed while it's made: the parts copied, each
// named apart from the borrowed value it's read of (#200).
use std::cell::RefCell;

fn main() {
    let outer = RefCell::new(RefCell::new((vec![1u8], 9u8)));
    let snap = outer.clone();
    outer.borrow().borrow_mut().0.push(2);
    println!("{outer:?} {snap:?}");

    let pairs = RefCell::new(vec![RefCell::new((String::from("a"), 1u32))]);
    let copied = pairs.clone();
    pairs.borrow()[0].borrow_mut().0.push('b');
    println!("{pairs:?} {copied:?} {}", pairs == copied);
}
