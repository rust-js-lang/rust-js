// A `while` followed by code that holds a `RefCell`'s borrow and returns,
// React's describeNode: `while (childFiber != null && idx < ..) { .. }`,
// then the borrow's `try { .. return content; } finally { $unborrow(..) }`
// after it, where the loop ends as Rust's does, not `while (true) { if
// (..) { ..; continue; } ..the rest.. }` (rd302).
use std::cell::RefCell;
use std::rc::Rc;

struct Fiber {
    sibling: RefCell<Option<Rc<Fiber>>>,
    name: String,
}

struct Node {
    fiber: Rc<Fiber>,
    children: RefCell<Vec<Rc<Node>>>,
    tail: RefCell<Vec<Rc<Node>>>,
}

fn describe(node: &Node, first: Option<Rc<Fiber>>, f: &dyn Fn(&Node) -> String) -> String {
    let mut content = String::new();
    let mut child_fiber = first;
    let mut idx = 0;
    while let Some(current) = child_fiber.clone()
        && idx < node.children.borrow().len()
    {
        let child = Rc::clone(&node.children.borrow()[idx]);
        if Rc::ptr_eq(&child.fiber, &current) {
            content += &describe(&child, None, f);
            idx += 1;
        } else {
            content += &current.name;
        }
        child_fiber = current.sibling.borrow().clone();
    }
    if child_fiber.is_some() {
        content += "...";
    }
    let tail = node.tail.borrow();
    for item in tail.iter() {
        content += &f(item);
    }
    content
}

fn fiber(name: &str, sibling: Option<Rc<Fiber>>) -> Rc<Fiber> {
    Rc::new(Fiber {
        sibling: RefCell::new(sibling),
        name: name.to_string(),
    })
}

fn leaf(fiber: Rc<Fiber>) -> Rc<Node> {
    Rc::new(Node {
        fiber,
        children: RefCell::new(Vec::new()),
        tail: RefCell::new(Vec::new()),
    })
}

fn main() {
    let c = fiber("c", None);
    let b = fiber("b", Some(Rc::clone(&c)));
    let a = fiber("a", Some(Rc::clone(&b)));
    let root = Node {
        fiber: fiber("root", None),
        children: RefCell::new(vec![leaf(Rc::clone(&b))]),
        tail: RefCell::new(vec![leaf(fiber("t1", None)), leaf(fiber("t2", None))]),
    };
    let named = |n: &Node| format!("<{}>", n.fiber.name);
    println!("{}", describe(&root, Some(Rc::clone(&a)), &named));
    println!("{}", describe(&root, None, &named));
    root.children.borrow_mut().push(leaf(Rc::clone(&c)));
    println!("{}", describe(&root, Some(a), &named));
    // The tail's borrow given back: it's borrowed mutably after.
    root.tail.borrow_mut().clear();
    println!("{}", describe(&root, Some(b), &|n| n.fiber.name.clone()));
}
