// An untagged enum's method whose every arm reads the same field of its
// variant's value, the test renderer's `Instance | Container` parent,
// whose `children` upstream reads of either, `parentInstance.children.push(child)`:
// that field's read, where it's called (rd317). One of a field that isn't
// each variant's, or of another key, is a method still.
use std::cell::{Cell, RefCell};

struct Container {
    children: RefCell<Vec<String>>,
    count: Cell<u32>,
    #[cfg_attr(rust_js, rust_js::test)]
    root_id: &'static str,
}

struct Instance {
    children: RefCell<Vec<String>>,
    count: Cell<u32>,
    tag: &'static str,
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Parent<'a> {
    Container(&'a Container),
    Instance(&'a Instance),
}

// Its callers first, which are lowered after it all the same.
fn append_child(parent_instance: &Parent, child: &str) {
    parent_instance.children().borrow_mut().push(child.to_string());
    parent_instance.count().set(parent_instance.count().get() + 1);
}

fn remove_child(parent_instance: &Parent, child: &str) {
    let index = parent_instance.children().borrow().iter().position(|c| c == child);
    if let Some(index) = index {
        parent_instance.children().borrow_mut().remove(index);
    }
}

impl Parent<'_> {
    fn children(&self) -> &RefCell<Vec<String>> {
        match self {
            Parent::Container(container) => &container.children,
            Parent::Instance(instance) => &instance.children,
        }
    }

    fn count(&self) -> &Cell<u32> {
        match self {
            Parent::Container(container) => &container.count,
            Parent::Instance(instance) => &instance.count,
        }
    }

    // Of each its own field: a method still.
    fn name(&self) -> &'static str {
        match self {
            Parent::Container(container) => container.root_id,
            Parent::Instance(instance) => instance.tag,
        }
    }
}

fn main() {
    let container = Container {
        children: RefCell::new(Vec::new()),
        count: Cell::new(0),
        root_id: "root",
    };
    let instance = Instance {
        children: RefCell::new(Vec::new()),
        count: Cell::new(0),
        tag: "div",
    };
    let parents = [Parent::Container(&container), Parent::Instance(&instance)];
    for parent in &parents {
        append_child(parent, "a");
        append_child(parent, "b");
        append_child(parent, "c");
        remove_child(parent, "b");
        println!(
            "{}: {:?}, {}",
            parent.name(),
            parent.children().borrow(),
            parent.count().get()
        );
    }
    println!("{:?} {:?}", container.children.borrow(), instance.children.borrow());
}
