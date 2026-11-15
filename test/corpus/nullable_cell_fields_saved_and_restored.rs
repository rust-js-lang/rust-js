// React Fizz's save-and-restore of a task's nullable cell fields: the old
// value a `RefCell`'s `replace` and `take` give back, and a `Cell`'s
// `get()`, each `null` or a value, restored as it was, `task.keyPath =
// prevKeyPath`, never `?? null`; and a `let mut` set first to
// what a nullable function gives, then to another's, returned as it
// is (#1933). Each restored where the work in between panicked too.
use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

pub struct KeyNode {
    pub name: &'static str,
}

pub struct Task {
    #[cfg_attr(rust_js, rust_js::nullable)]
    key_path: RefCell<Option<Rc<KeyNode>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    row: RefCell<Option<Rc<KeyNode>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    preamble: Cell<Option<&'static KeyNode>>,
    id: u32,
}

static OUTER: KeyNode = KeyNode { name: "outer" };

fn described(task: &Task) -> String {
    let name = |node: Option<&KeyNode>| node.map_or("none", |node| node.name);
    format!(
        "{} key {} row {} preamble {}",
        task.id,
        name(task.key_path.borrow().as_deref()),
        name(task.row.borrow().as_deref()),
        name(task.preamble.get())
    )
}

fn render(task: &Rc<Task>, key: &Rc<KeyNode>, panics: bool) -> bool {
    let prev_key_path = task.key_path.replace(Some(Rc::clone(key)));
    let prev_row = task.row.take();
    let prev_preamble = task.preamble.get();
    task.preamble.set(Some(&OUTER));
    let result = catch_unwind(AssertUnwindSafe(|| {
        println!("rendering {}", described(task));
        if panics {
            panic!("render failed");
        }
    }));
    task.key_path.replace(prev_key_path);
    task.row.replace(prev_row);
    task.preamble.set(prev_preamble);
    result.is_ok()
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn found(task: &Task, wanted: u32) -> Option<Rc<KeyNode>> {
    match task.id == wanted {
        true => task.key_path.borrow().clone(),
        false => None,
    }
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn nothing() -> Option<Rc<KeyNode>> {
    None
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn active(task: &Task, wanted: u32, look: bool) -> Option<Rc<KeyNode>> {
    let mut active = nothing();
    if look {
        active = found(task, wanted);
    }
    active
}

fn main() {
    let row = Rc::new(KeyNode { name: "row" });
    let task = Rc::new(Task {
        key_path: RefCell::new(None),
        row: RefCell::new(Some(Rc::clone(&row))),
        preamble: Cell::new(None),
        id: 7,
    });
    let key = Rc::new(KeyNode { name: "key" });
    println!("before {}", described(&task));
    println!("ok {}", render(&task, &key, false));
    println!("after {}", described(&task));
    println!("ok {}", render(&task, &key, true));
    println!("after panic {}", described(&task));
    task.key_path.replace(Some(Rc::clone(&key)));
    let inner = Rc::new(KeyNode { name: "inner" });
    println!("ok {}", render(&task, &inner, false));
    println!("after {}", described(&task));
    for (wanted, look) in [(7, true), (8, true), (7, false)] {
        let got = active(&task, wanted, look);
        println!("active {wanted} {look}: {}", got.as_deref().map_or("none", |node| node.name));
    }
}
