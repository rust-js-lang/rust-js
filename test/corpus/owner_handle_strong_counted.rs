// A cell of a root's `Rc` that a callback made before the root reads,
// where the root's `Rc` is counted: the cell's clone of it is a count,
// which `Rc::strong_count` shows, so the cell is a cell, as written, not
// the root's `let` (#577, ADR 0317, amended).

use std::cell::OnceCell;
use std::rc::Rc;

struct Root {
    name: String,
    on_commit: Rc<dyn Fn()>,
}

fn create_container(name: &str, on_commit: Rc<dyn Fn()>) -> Rc<Root> {
    Rc::new(Root {
        name: name.to_string(),
        on_commit,
    })
}

fn commit(root: &Root) {
    let on_commit = &root.on_commit;
    on_commit();
}

fn counted(name: &str) -> Rc<Root> {
    let created: Rc<OnceCell<Rc<Root>>> = Rc::default();
    let on_commit: Rc<dyn Fn()> = {
        let created = Rc::clone(&created);
        Rc::new(move || match created.get() {
            Some(root) => println!("committed {}, held {} times", root.name, Rc::strong_count(root)),
            None => println!("committed before the root is set"),
        })
    };
    let root = create_container(name, on_commit);
    let _ = created.set(Rc::clone(&root));
    println!("{} is held {} times", root.name, Rc::strong_count(&root));
    root
}

fn main() {
    commit(&counted("g"));
}
