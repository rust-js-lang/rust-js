// A closure holding a type parameter's function, made a `dyn Fn`, holds
// what its callers give it (ADR 0521, amended by #580): where one gives
// the parameter a closure holding a value with a destructor, the `dyn Fn`
// drops it where Rust does, when the last `Rc` of the node holding it is
// dropped, and one given a closure holding nothing drops nothing.
use std::rc::Rc;

struct Guard(u32);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop guard {}", self.0);
    }
}

impl Guard {
    fn id(&self) -> u32 {
        self.0
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
    let plain = computed(|previous| previous * 2);
    println!("plain {}", (plain.getter)(3));
    let guard = Guard(2);
    let guarded = computed(move |previous| previous + guard.id());
    let shared = Rc::clone(&guarded);
    println!("guarded {}", (guarded.getter)(1));
    drop(guarded);
    println!("one left");
    println!("shared {}", (shared.getter)(5));
    drop(shared);
    println!("none left");
    drop(plain);
    println!("done");
}
