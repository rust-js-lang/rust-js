// An `Rc` of a closure made an `Rc<dyn Fn>` is one allocation: where the
// closure's `Rc` is counted, here as its count is read, the `dyn Fn`'s is
// too, so a call through it reads its `value`; it was called as the
// function, `TypeError: callback is not a function` (#458). A library
// counted it as one holding a type parameter's value, its type naming
// `make`'s `impl Fn`, which it holds none of: neither build does now
// (rd597).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn make(run: impl Fn(&mut dyn FnMut(&str))) -> Vec<Rc<dyn Fn(&str) -> String>> {
    let mut made: Vec<Rc<dyn Fn(&str) -> String>> = Vec::new();
    run(&mut |prefix| {
        let prefix = prefix.to_string();
        let callback = Rc::new(move |v: &str| format!("{prefix} {v}"));
        println!("{}", Rc::strong_count(&callback));
        made.push(callback);
    });
    made
}

pub struct Chunk {
    info: RefCell<String>,
    seen: Cell<u32>,
}

impl Chunk {
    pub fn watch(self: &Rc<Self>, run: impl Fn(&mut dyn FnMut(&str))) -> Vec<Rc<dyn Fn(&str) -> String>> {
        let mut made: Vec<Rc<dyn Fn(&str) -> String>> = Vec::new();
        run(&mut |prefix| {
            let chunk = Rc::clone(self);
            let prefix = prefix.to_string();
            made.push(Rc::new(move |value| {
                chunk.seen.set(chunk.seen.get() + 1);
                format!("{prefix} {value} {}", chunk.info.borrow())
            }));
        });
        made
    }
}

fn main() {
    let made = make(|f| {
        f("a");
        f("b");
    });
    for callback in &made {
        println!("{}", callback("x"));
    }
    let chunk = Rc::new(Chunk {
        info: RefCell::new("i".into()),
        seen: Cell::new(0),
    });
    let made = chunk.watch(|f| f("c"));
    let shared = Rc::clone(&made[0]);
    println!("{} {}", shared("y"), made[0]("z"));
    println!("{}", chunk.seen.get());
}
