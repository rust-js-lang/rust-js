// A `let` that ends a `RefCell`'s borrow, named as the field it reads by
// their Rust names, is read in place by the store after it whatever their JS
// names (ADR 0866, amended): `r#return`'s `return$` of `.return`, React's
// `sibling.return = node.return;`, and a field of a `#[rust_js::name]`,
// `workInProgress._debugHookTypes = current._debugHookTypes;`. One named
// otherwise keeps its name.
use std::cell::RefCell;
use std::rc::Rc;

struct Node {
    name: &'static str,
    r#return: RefCell<Option<Rc<Node>>>,
    #[cfg_attr(rust_js, rust_js::name = "_debugHookTypes")]
    debug_hook_types: RefCell<Option<Rc<Vec<u32>>>>,
}

fn node(name: &'static str) -> Rc<Node> {
    Rc::new(Node {
        name,
        r#return: RefCell::new(None),
        debug_hook_types: RefCell::new(None),
    })
}

fn adopt(node: &Node, sibling: &Node) {
    let r#return = node.r#return.borrow().clone();
    sibling.r#return.replace(r#return);
}

fn copy_debug(work_in_progress: &Node, current: &Node) {
    let debug_hook_types = current.debug_hook_types.borrow().clone();
    work_in_progress.debug_hook_types.replace(debug_hook_types);
    let parent = current.r#return.borrow().clone();
    work_in_progress.r#return.replace(parent);
}

fn main() {
    let root = node("root");
    let first = node("first");
    first.r#return.replace(Some(Rc::clone(&root)));
    first.debug_hook_types.replace(Some(Rc::new(vec![1, 2])));
    let second = node("second");
    adopt(&first, &second);
    let second = (second.name, second.r#return.borrow().as_ref().map(|r| r.name));
    println!("{second:?}");
    let work = node("work");
    copy_debug(&work, &first);
    let parent = work.r#return.borrow().as_ref().map(|r| r.name);
    println!("{:?} {parent:?}", work.debug_hook_types.borrow());
}
