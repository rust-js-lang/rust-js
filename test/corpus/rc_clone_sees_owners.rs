// `make_mut` and `unwrap_or_clone` of a shared `Rc` clone what it points at
// while it's still shared: the clone sees every owner, as std's does, and
// then drop the `Rc`, the last of which drops what it points at.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

thread_local! {
    static KEPT: RefCell<Option<Rc<Lone>>> = const { RefCell::new(None) };
}

/// Drops the only other owner as it's cloned.
struct Lone(u8);

impl Clone for Lone {
    fn clone(&self) -> Self {
        KEPT.with_borrow_mut(|kept| *kept = None);
        Lone(self.0 + 1)
    }
}

impl Drop for Lone {
    fn drop(&mut self) {
        println!("drop lone {}", self.0);
    }
}

struct Probe {
    me: Weak<Probe>,
}

impl Clone for Probe {
    fn clone(&self) -> Self {
        println!("owners while cloning: {}", self.me.strong_count());
        Probe { me: Weak::new() }
    }
}

fn main() {
    let mut first = Rc::new_cyclic(|me| Probe { me: me.clone() });
    let second = Rc::clone(&first);
    Rc::make_mut(&mut first);
    println!("{}", Rc::strong_count(&second));
    let third = Rc::clone(&second);
    let _owned: Probe = Rc::unwrap_or_clone(third);
    println!("{}", Rc::strong_count(&second));
    let mut lone = Rc::new(Lone(1));
    KEPT.with_borrow_mut(|kept| *kept = Some(Rc::clone(&lone)));
    Rc::make_mut(&mut lone);
    println!("made {}", lone.0);
    let shared = Rc::new(Lone(5));
    KEPT.with_borrow_mut(|kept| *kept = Some(Rc::clone(&shared)));
    let owned = Rc::unwrap_or_clone(shared);
    println!("owned {}", owned.0);
}
