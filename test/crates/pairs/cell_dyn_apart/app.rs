//! A `RefCell<Vec<Rc<Node>>>` of the app's isn't the library's checked
//! `RefCell<Vec<Rc<dyn Fn()>>>`: a `Vec`'s `Rc<dyn Fn()>` is no `Rc<Node>`
//! (#374, ADR 0960), so its borrows aren't checked.
use std::cell::RefCell;
use std::rc::Rc;

pub struct Node {
    pub name: String,
    pub children: RefCell<Vec<Rc<Node>>>,
}

fn node(name: &str) -> Rc<Node> {
    Rc::new(Node {
        name: name.to_string(),
        children: RefCell::new(Vec::new()),
    })
}

fn push(siblings: &RefCell<Vec<Rc<Node>>>, child: Rc<Node>) {
    siblings.borrow_mut().push(child);
}

pub fn main() {
    let root = node("root");
    push(&root.children, node("child"));
    let hooks = dep::hooks();
    let seen = Rc::clone(&root);
    dep::add(
        &hooks,
        Rc::new(move || println!("{} {}", seen.name, seen.children.borrow().len())),
    );
    dep::run(&hooks);
    push(&root.children, node("other"));
    dep::run(&hooks);
}
