// A guard that's a call's receiver, held while its argument drops a value
// of a type parameter, whose every instance's destructor asks nothing of
// the guard's cell: the instances its callers make are each seen, so the
// borrow isn't counted (#2490, #1321), as one of a type that may be any
// would be.
use std::cell::RefCell;

struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn push_after<T>(it: T, counts: &RefCell<Vec<u8>>) {
    counts.borrow_mut().push({
        let _it = it;
        1
    });
}

fn main() {
    let counts = RefCell::new(Vec::new());
    push_after(Noisy(1), &counts);
    push_after(Noisy(2), &counts);
    println!("{:?}", counts.borrow());
}
