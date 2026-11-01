// A clone a closure makes of a parameter it borrowed, to move into a closure
// of its own, as React's Flight client's DEV `then` shares its chunk with the
// callbacks its executor makes, `let chunk = Rc::clone(self)`: the same JS
// object as the parameter, which the function never gives another value,
// read by the parameter's name, `this` in a method, as upstream's
// `this._debugInfo` (ADRs 0846, 0879, amended, rd515). A `mut` parameter
// given another value after keeps its clone's name.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::class)]
pub struct Chunk {
    info: RefCell<String>,
    seen: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Chunk {
    pub fn new(info: &str) -> Self {
        Chunk {
            info: RefCell::new(info.to_string()),
            seen: Cell::new(0),
        }
    }

    // The executor borrows `self`; each callback it makes holds a clone.
    pub fn watch(self: &Rc<Self>, run: impl Fn(&mut dyn FnMut(&str))) -> Vec<Box<dyn Fn(&str) -> String>> {
        let mut made: Vec<Box<dyn Fn(&str) -> String>> = Vec::new();
        let calls: Rc<Cell<u32>> = Rc::default();
        run(&mut |prefix| {
            let chunk = Rc::clone(self);
            let prefix = prefix.to_string();
            made.push(Box::new(move |value| {
                chunk.seen.set(chunk.seen.get() + 1);
                format!("{prefix} {value} {}", chunk.info.borrow())
            }));
            // A tuple of clones, as React's `then` makes its two.
            let (chunk, counted) = (Rc::clone(self), Rc::clone(&calls));
            made.push(Box::new(move |value| {
                counted.set(counted.get() + 1);
                chunk.info.borrow_mut().push_str(value);
                format!("{} {}", chunk.info.borrow(), counted.get())
            }));
        });
        made
    }
}

struct Payload {
    status: Cell<i32>,
}

// A plain function's parameter, borrowed by the closure that clones it.
fn settle(payload: &Rc<Payload>, run: impl Fn(&mut dyn FnMut(i32))) -> Vec<Box<dyn Fn() -> i32>> {
    let mut made: Vec<Box<dyn Fn() -> i32>> = Vec::new();
    run(&mut |status| {
        let resolved = Rc::clone(payload);
        made.push(Box::new(move || {
            resolved.status.set(status);
            resolved.status.get()
        }));
    });
    made
}

// A loop's `let`, made again each turn, each callback holding its turn's;
// and one shadowed after its clone is made, which the callback still holds.
fn each_turn() -> Vec<i32> {
    let mut made: Vec<Box<dyn Fn() -> i32>> = Vec::new();
    for i in 0..3 {
        let a = Rc::new(i);
        let mut make = || {
            let b = Rc::clone(&a);
            made.push(Box::new(move || *b * 10));
        };
        make();
    }
    let a = Rc::new(7);
    let make = || {
        let b = Rc::clone(&a);
        move || *b
    };
    let g = make();
    let a = Rc::new(8);
    made.push(Box::new(move || *a + g()));
    made.iter().map(|f| f()).collect()
}

// A `mut` parameter, given another value after its clone is made: the
// clone keeps its own name, the value it was made of.
fn given_another(mut a: Rc<i32>) -> i32 {
    let make = || {
        let b = Rc::clone(&a);
        move || *b
    };
    let g = make();
    a = Rc::new(2);
    *a * 10 + g()
}

fn main() {
    let chunk = Rc::new(Chunk::new("i"));
    let made = chunk.watch(|f| {
        f("a");
        f("b");
    });
    for callback in &made {
        println!("{}", callback("x"));
    }
    println!("{} {}", chunk.seen.get(), chunk.info.borrow());

    let payload = Rc::new(Payload { status: Cell::new(0) });
    let made = settle(&payload, |f| {
        f(3);
        f(4);
    });
    let results: Vec<i32> = made.iter().map(|f| f()).collect();
    println!("{results:?} {}", payload.status.get());

    println!("{:?}", each_turn());
    println!("{}", given_another(Rc::new(1)));
}
