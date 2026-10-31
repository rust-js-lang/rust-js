// A loop's value given at its one way out, `let next = loop { .. if let
// Some(next) = node.next.clone() { break next; } .. }`, is given in the
// test that leaves, `if ((next = node.next) != null) { break; }`, as React's
// getModernOffsetsFromPoints walks to a next sibling (rd148; ADR 0702,
// amended). Not where the test reads its value twice, `next != null &&
// next.value > 1` or `v > Math.min(limit, v)`.
use std::cell::Cell;
use std::rc::Rc;

struct Node {
    value: u32,
    next: Option<Rc<Node>>,
    parent: Option<Rc<Node>>,
}

fn walk(mut node: Rc<Node>, stop: u32) -> u32 {
    let mut total = 0;
    'outer: loop {
        let next = loop {
            if node.value == stop {
                break 'outer;
            }
            total += node.value;
            if let Some(next) = node.next.clone() {
                break next;
            }
            match node.parent.clone() {
                Some(parent) => node = parent,
                None => break 'outer,
            }
        };
        node = next;
    }
    total
}

fn either(mut node: Rc<Node>, stop: u32) -> u32 {
    let mut total = 0;
    loop {
        let next = loop {
            if node.value == stop {
                break None;
            }
            total += node.value;
            if let Some(next) = node.next.clone() {
                break Some(next);
            }
            match node.parent.clone() {
                Some(parent) => node = parent,
                None => return total,
            }
        };
        match next {
            Some(next) => node = next,
            None => return total * 10,
        }
    }
}

fn deeper(mut node: Rc<Node>) -> u32 {
    let next = loop {
        if let Some(next) = node.next.clone()
            && next.value > 1
        {
            break next;
        }
        match node.parent.clone() {
            Some(parent) => node = parent,
            None => return 0,
        }
    };
    next.value
}

fn node(value: u32, next: Option<Rc<Node>>, parent: Option<Rc<Node>>) -> Rc<Node> {
    Rc::new(Node { value, next, parent })
}

fn bump(count: &Cell<u32>) -> u32 {
    count.set(count.get() + 1);
    count.get()
}

fn first_over(count: &Cell<u32>, limit: u32) -> u32 {
    let found = loop {
        let v = bump(count);
        if v > limit.min(v) {
            break v;
        }
    };
    found * 100 + count.get()
}

fn top_of(node: &Rc<Node>) -> Rc<Node> {
    node.parent.clone().unwrap_or_else(|| Rc::clone(node))
}

fn main() {
    let top = node(100, None, None);
    let last = node(3, None, Some(Rc::clone(&top)));
    let second = node(2, Some(last), Some(Rc::clone(&top)));
    let first = node(1, Some(second), Some(top));
    println!("{} {} {}", walk(Rc::clone(&first), 3), walk(Rc::clone(&first), 7), walk(Rc::clone(&first), 2));
    println!("{} {}", deeper(Rc::clone(&first)), deeper(Rc::clone(&top_of(&first))));
    println!("{}", first_over(&Cell::new(0), 3));
    println!("{} {}", either(Rc::clone(&first), 3), either(Rc::clone(&first), 7));
}
