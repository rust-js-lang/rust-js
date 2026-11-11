// A counted `Rc` of a `#[rust_js::class]`, the instance itself (#1228):
// given back by `try_unwrap` and counted anew by `Rc::new`, a new
// allocation, which a `Weak` of the old one never upgrades to; `into_inner`,
// `get_mut` of the only one, `new_cyclic` given a `Weak` to the instance it
// makes, `Weak::new()`, and generic code reading what it points at, each as
// native Rust does.
use std::cell::Cell;
use std::rc::{Rc, Weak};

#[cfg_attr(rust_js, rust_js::class)]
struct Node {
    n: Cell<u32>,
    me: Weak<Node>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Node {
    fn new(n: u32, me: Weak<Node>) -> Self {
        Node { n: Cell::new(n), me }
    }

    fn get(&self) -> u32 {
        self.n.get()
    }

    fn knows_itself(&self) -> bool {
        self.me.upgrade().is_some_and(|me| me.get() == self.get())
    }
}

fn read<T, R>(rc: &Rc<T>, f: impl Fn(&T) -> R) -> R {
    f(rc)
}

fn main() {
    let first = Rc::new(Node::new(1, Weak::new()));
    let weak = Rc::downgrade(&first);
    let node = Rc::try_unwrap(first).ok().expect("the only one");
    println!("{} {}", node.get(), weak.upgrade().is_none());
    let again = Rc::new(node);
    println!(
        "{} {} {}",
        weak.upgrade().is_none(),
        Rc::strong_count(&again),
        read(&again, |n| n.get())
    );
    let shared = Rc::clone(&again);
    let mut only = again;
    println!("{}", Rc::get_mut(&mut only).is_none());
    drop(shared);
    if let Some(node) = Rc::get_mut(&mut only) {
        node.n.set(7);
    }
    println!("{} {:?}", only.get(), Rc::into_inner(only).map(|n| n.get()));
    let cyclic = Rc::new_cyclic(|me| Node::new(9, me.clone()));
    println!(
        "{} {} {}",
        cyclic.knows_itself(),
        Rc::weak_count(&cyclic),
        Weak::ptr_eq(&cyclic.me, &Rc::downgrade(&cyclic))
    );
    let none: Weak<Node> = Weak::new();
    println!("{} {}", none.upgrade().is_none(), none.strong_count());
}
