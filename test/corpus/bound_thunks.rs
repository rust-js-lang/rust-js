// A `move` closure of no parameters of its own that passes on what it took
// by value to a function of the crate's is that function bound, as
// alien-signals' `computedOper.bind(null, node)` (#502, ADR 0605,
// amended): `bind` reads what it binds where the closure is made, as the
// closure takes it. A variable written after the closure took its copy, a
// node the function changes through its cell, a thunk called again, given
// back, held, or made where an `Option`'s `map` calls it, each gives what
// Rust does. One boxed, a callback handed on, one that borrows what it
// passes, and one that makes its call a statement stay arrows.

use std::cell::Cell;
use std::rc::Rc;

struct Node {
    name: String,
    runs: Cell<u32>,
}

fn computed_oper(node: &Rc<Node>) -> String {
    node.runs.set(node.runs.get() + 1);
    format!("{} {}", node.name, node.runs.get())
}

fn shown(n: u32, tag: &str) -> String {
    format!("{tag}{n}")
}

fn effect_oper(node: &Rc<Node>) {
    node.runs.set(node.runs.get() * 10);
}

fn computed(node: Rc<Node>) -> impl Fn() -> String {
    move || computed_oper(&node)
}

fn effect(node: Rc<Node>) -> Box<dyn Fn()> {
    Box::new(move || {
        effect_oper(&node);
    })
}

fn main() {
    let node = Rc::new(Node {
        name: "a".to_string(),
        runs: Cell::new(0),
    });
    let read = computed(Rc::clone(&node));
    println!("{} / {}", read(), read());

    let mut n = 3;
    let later = move || shown(n, "n=");
    n += 1;
    println!("{} {}", later(), n);

    let thunks: Vec<Box<dyn Fn() -> String>> = (0..3)
        .map(|i| -> Box<dyn Fn() -> String> { Box::new(move || shown(i, "#")) })
        .collect();
    let all: Vec<String> = thunks.iter().map(|f| f()).collect();
    println!("{all:?}");

    let maybe = Some(Rc::clone(&node)).map(|node| move || computed_oper(&node));
    println!("{:?}", maybe.map(|f| f()));

    effect(Rc::clone(&node))();
    println!("runs {}", node.runs.get());

    let peek = || computed_oper(&node);
    println!("{} {}", peek(), peek());
}
