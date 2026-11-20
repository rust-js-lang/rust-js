//@ run-pass
// An array of arrays matched by value in a closure, its or-patterns binding
// one element of each inner array: what the bindings leave is dropped as the
// arm ends, each element as rustc's drop elaboration takes the arrays apart,
// then the bindings. rustc's tests/ui/drop/or-pattern-drop-order.rs's
// shape, which change D's (c) refused while the drop shadow couldn't take an
// array apart (#2177).
use std::cell::RefCell;

struct Loud<'a>(&'a RefCell<Vec<u32>>, u32);

impl Drop for Loud<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

fn main() {
    let order = RefCell::new(Vec::new());
    let run = |o: &RefCell<Vec<u32>>| match [
        [Loud(o, 6), Loud(o, 4)],
        [Loud(o, 3), Loud(o, 2)],
        [Loud(o, 1), Loud(o, 5)],
    ] {
        [[_, w] | [w, _], [x, y] | [y, x], [z, _] | [_, z]] if w.1 > 0 => {
            println!("bound {} {} {} {}", w.1, x.1, y.1, z.1);
        }
        _ => unreachable!(),
    };
    run(&order);
    println!("{:?}", order.borrow());
}
