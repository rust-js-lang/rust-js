//@ library-differs
// A clone read only through its fields is never a value JS gets whole
// (#2408, ADR 1197): `let (node, depth) = stack[at].clone()` of an
// `(Rc<Node>, usize)`, whose `Rc` reaches JS in the `Vec` returned, copies
// nothing, as an `Rc`'s clone shares and a number isn't an object: its JS
// reads the parts, `let [node, depth] = stack[at]`, as React's
// `findPaths` does. The same tuple returned whole is JS's to change, so a
// copy of its own, `[pair[0], pair[1]]`: JS changing it leaves the stack's.
use std::rc::Rc;

pub struct Node {
    pub name: String,
    pub children: Vec<Rc<Node>>,
}

pub fn deep(root: &Rc<Node>, below: usize) -> Vec<Rc<Node>> {
    let mut found = Vec::new();
    let mut stack = vec![(Rc::clone(root), 0)];
    let mut at = 0;
    while at < stack.len() {
        let (node, depth) = stack[at].clone();
        at += 1;
        if depth >= below {
            found.push(Rc::clone(&node));
        }
        for child in &node.children {
            stack.push((Rc::clone(child), depth + 1));
        }
    }
    found
}

pub fn first_pair(pairs: &[(Rc<Node>, usize)]) -> (Rc<Node>, usize) {
    pairs[0].clone()
}

fn leaf(name: &str) -> Rc<Node> {
    Rc::new(Node {
        name: name.to_string(),
        children: Vec::new(),
    })
}

fn main() {
    let root = Rc::new(Node {
        name: "root".to_string(),
        children: vec![leaf("a"), leaf("b")],
    });
    let names: Vec<String> = deep(&root, 1).iter().map(|node| node.name.clone()).collect();
    println!("{names:?}");
    let pairs = vec![(Rc::clone(&root), 3)];
    let (node, depth) = first_pair(&pairs);
    println!("{} {depth} {}", node.name, pairs[0].1);
}
