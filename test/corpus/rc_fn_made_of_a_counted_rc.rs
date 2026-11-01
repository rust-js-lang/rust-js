//@ library-differs
// An `Rc` of a closure made an `Rc<dyn Fn>` is one allocation: where the
// closure's `Rc` is counted, as a library counts one holding what it may
// drop, the `dyn Fn`'s is too, so a call through it reads its `value`; it
// was called as the function, `TypeError: callback is not a function`
// (#458). The program build counts neither.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn make(run: impl Fn(&mut dyn FnMut(&str))) -> Vec<Rc<dyn Fn(&str) -> String>> {
    let mut made: Vec<Rc<dyn Fn(&str) -> String>> = Vec::new();
    run(&mut |prefix| {
        let prefix = prefix.to_string();
        made.push(Rc::new(move |v| format!("{prefix} {v}")));
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
