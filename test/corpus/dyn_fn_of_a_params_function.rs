//@ library-differs
// A closure holding a type parameter's function, made a `dyn Fn`, holds
// what its callers give it (ADR 0521, amended by #580): in a crate with a
// destructor, where none of them gives the parameter one whose drop runs,
// the `dyn Fn` drops nothing, so the `Rc` of the node holding it is the
// node itself, as alien-signals' `computed<T>` wraps its getter.
use std::rc::Rc;

struct Guard(u32);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop guard {}", self.0);
    }
}

struct Node {
    getter: Box<dyn Fn(u32) -> u32>,
}

fn computed<F: Fn(u32) -> u32 + 'static>(getter: F) -> Rc<Node> {
    Rc::new(Node {
        getter: Box::new(move |previous| getter(previous) + 1),
    })
}

fn main() {
    let _guard = Guard(1);
    let node = computed(|previous| previous * 2);
    let shared = Rc::clone(&node);
    println!("doubled {}", (shared.getter)(3));
    let offset = 10;
    let other = computed(move |previous| previous + offset);
    println!("offset {}", (other.getter)((node.getter)(1)));
    drop(shared);
    println!("done");
}
