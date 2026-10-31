// An untagged enum of structs, each told apart by a member its struct
// declares, `#[rust_js::test]`, as React's noop renderer tells an instance
// from a text instance, `isArray(child.children)`, and a container from an
// instance, `typeof parent.rootID === "string"` (rd291).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Instance {
    kind: String,
    parent: Cell<f64>,
    #[cfg_attr(rust_js, rust_js::test)]
    children: RefCell<Vec<Child>>,
}

struct TextInstance {
    text: RefCell<String>,
    parent: Cell<f64>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Child {
    Instance(Rc<Instance>),
    Text(Rc<TextInstance>),
}

struct Container {
    #[cfg_attr(rust_js, rust_js::name = "rootID", rust_js::test)]
    root_id: String,
    children: RefCell<Vec<Child>>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Parent {
    Instance(Rc<Instance>),
    Container(Rc<Container>),
}

fn parent_of(child: &Child) -> f64 {
    match child {
        Child::Instance(instance) => instance.parent.get(),
        Child::Text(text) => text.parent.get(),
    }
}

fn describe(child: &Child) -> String {
    match child {
        Child::Instance(instance) => {
            let inner: Vec<String> = instance.children.borrow().iter().map(describe).collect();
            format!("<{}>{}</{}>", instance.kind, inner.join(""), instance.kind)
        }
        Child::Text(text) => text.text.borrow().clone(),
    }
}

fn append(parent: &Parent, child: Child) {
    match parent {
        Parent::Instance(instance) => {
            println!("into <{}>", instance.kind);
            instance.children.borrow_mut().push(child);
        }
        Parent::Container(container) => {
            println!("into {}", container.root_id);
            container.children.borrow_mut().push(child);
        }
    }
}

fn main() {
    let div = Rc::new(Instance {
        kind: "div".to_string(),
        parent: Cell::new(1.0),
        children: RefCell::new(Vec::new()),
    });
    let text = Rc::new(TextInstance {
        text: RefCell::new("hi".to_string()),
        parent: Cell::new(2.0),
    });
    let root = Rc::new(Container {
        root_id: "root".to_string(),
        children: RefCell::new(Vec::new()),
    });
    append(&Parent::Instance(div.clone()), Child::Text(text.clone()));
    append(&Parent::Container(root.clone()), Child::Instance(div.clone()));
    println!("{} {}", parent_of(&Child::Instance(div.clone())), parent_of(&Child::Text(text)));
    for child in root.children.borrow().iter() {
        println!("{} {}", describe(child), matches!(child, Child::Text(_)));
    }
}
