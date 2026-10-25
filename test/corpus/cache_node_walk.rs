// React's `cache`, walking its nodes: a reference reassigned in a loop,
// `node = ..`, whose `Cell`s are read and set by `get` and `set` before it
// is, read where they are, `node.s`, with nothing holding what `node` was;
// a handle on one kept past the reassignment, or past a closure's, still
// reads the node it was made of. A cache filled the first time it's read,
// `let cache = match node.o.get() { None => { ..; cache } Some(cache) =>
// cache }`, one variable; `node = match .. { None => { let node = ..; ..;
// node } .. }`, the node given `node` where it's made; and a closure's
// loop variable shadowed by its own value, that variable (ADR 0849).
use std::cell::Cell;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Status {
    Unterminated,
    Terminated,
}

struct Node {
    s: Cell<Status>,
    v: Cell<u32>,
    o: Cell<Option<&'static Cell<u32>>>,
    next: Cell<Option<&'static Node>>,
}

fn create_node() -> &'static Node {
    Box::leak(Box::new(Node {
        s: Cell::new(Status::Unterminated),
        v: Cell::new(0),
        o: Cell::new(None),
        next: Cell::new(None),
    }))
}

fn lookup(keys: &[u32], start: Option<&'static Node>) -> u32 {
    let mut node = start.unwrap_or_else(create_node);
    for &k in keys {
        let cache = match node.o.get() {
            None => {
                let cache: &'static Cell<u32> = Box::leak(Box::new(Cell::new(k)));
                node.o.set(Some(cache));
                cache
            }
            Some(cache) => cache,
        };
        cache.set(cache.get() + 1);
        node = match node.next.get() {
            None => {
                let node = create_node();
                node.v.set(k);
                node
            }
            Some(next) => next,
        };
    }
    if node.s.get() == Status::Terminated {
        return node.v.get();
    }
    node.s.set(Status::Terminated);
    node.v.set(node.v.get() + 100);
    node.v.get()
}

// A handle made before the reference is reassigned, read after: the
// node it was made of.
fn kept(a: &'static Node, b: &'static Node) -> (u32, u32) {
    let mut node = a;
    let v = &node.v;
    node = b;
    v.set(v.get() + 1);
    let first = v.get();
    let mut node2 = a;
    let w = &node2.v;
    let mut swap = || node2 = b;
    swap();
    w.set(w.get() + 10);
    (first + node.v.get(), w.get() + node2.v.get())
}

// A `continue` before the variable's given what's made: it's given
// nothing that turn.
fn skipping(keys: &[u32]) -> u32 {
    let mut node = create_node();
    for &k in keys {
        node = {
            let node = create_node();
            node.v.set(k);
            if k % 2 == 0 {
                continue;
            }
            node
        };
    }
    node.v.get()
}

// A closure reading the variable, run before it's given what's made:
// what it was. A cache filled by `if let`, and one by `match` of a binding
// the program names, which stays.
fn shown(a: &'static Node) -> u32 {
    let mut node = a;
    let show = || node.v.get();
    node = {
        let node = create_node();
        node.v.set(show() + 7);
        node
    };
    let cache = if let Some(cache) = node.o.get() {
        cache
    } else {
        let cache: &'static Cell<u32> = Box::leak(Box::new(Cell::new(1)));
        node.o.set(Some(cache));
        cache
    };
    let found = node.next.get();
    let next = match found {
        Some(found) => found,
        None => {
            let next = create_node();
            node.next.set(Some(next));
            next
        }
    };
    node.v.get() + cache.get() + next.v.get()
}

fn longest() -> impl Fn(&[String]) -> usize {
    |names: &[String]| {
        let mut most = 0;
        for name in names {
            let name: &str = name;
            most = most.max(name.len());
        }
        most
    }
}

pub fn main() {
    println!("{}", lookup(&[1, 2], None));
    let n = create_node();
    println!("{}", lookup(&[], Some(n)));
    println!("{}", lookup(&[], Some(n)));
    println!("{:?} {}", n.s.get(), n.v.get());
    let a = create_node();
    let b = create_node();
    b.v.set(5);
    println!("{:?}", kept(a, b));
    println!("{}", skipping(&[3, 4, 6]));
    println!("{}", shown(b));
    let names = vec![String::from("ab"), String::from("abcd")];
    println!("{}", longest()(&names));
}
