// An or-pattern of a constant first field takes its first alternative,
// whose bindings drop in its order, `y` then `x`: the `if (true)` MIR
// tests it by is its branch, in its place (#247).
use std::cell::RefCell;

struct Logged<'o>(&'o RefCell<Vec<u32>>, u32);

impl Drop for Logged<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

fn main() {
    let order = RefCell::new(Vec::new());
    match (true, Logged(&order, 2), Logged(&order, 1)) {
        (true, x, y) | (false, y, x) => println!("{} {}", x.1, y.1),
    }
    let ((true, x, y) | (false, y, x)) = (false, Logged(&order, 3), Logged(&order, 4));
    println!("{} {}", x.1, y.1);
    drop(x);
    drop(y);
    println!("{:?}", order.borrow());
}
