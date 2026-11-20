//@ run-pass
// A field moved out of a pair a struct holds in a `Box`: what's left of the
// pair is dropped as the struct is, then the moved field, and the box is
// freed by rustc's call, not a `Drop`, so the drop shadow can't take its
// contents apart and counts the rest unmatched, never a disagreement
// (#2177); change D's (c) refused it while it did.
use std::cell::RefCell;

struct Loud<'a>(&'a RefCell<Vec<u32>>, u32);

impl Drop for Loud<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

struct Pair<'a> {
    left: Loud<'a>,
    right: Box<(Loud<'a>, Loud<'a>)>,
}

fn main() {
    let order = RefCell::new(Vec::new());
    let o = &order;
    {
        let pair = Pair {
            left: Loud(o, 1),
            right: Box::new((Loud(o, 2), Loud(o, 3))),
        };
        let moved = pair.right.0;
        println!("moved {} beside {}", moved.1, pair.left.1);
    }
    println!("{:?}", order.borrow());
}
