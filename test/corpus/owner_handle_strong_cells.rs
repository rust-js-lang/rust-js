// Cells of a root's `Rc` that a callback made before the root reads, each
// one the root's `let` can't be (#577): something reads the cell between
// the call and its setting, where it's `None` and the variable would hold
// the root; it's set only on one path, or on each turn of a loop, or of
// another root than the call's; or what the `Rc` holds may be `undefined`,
// an `Option` or a struct of no fields, which its `Some` boxes. Each is a
// cell, as written (ADR 0317, amended); one whose root's `Rc` is counted,
// `owner_handle_strong_counted`.

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

// Committed between the call and the cell's setting: `None` there.
fn committed_between(name: &str) -> Rc<Root> {
    let created: Rc<OnceCell<Rc<Root>>> = Rc::default();
    let on_commit: Rc<dyn Fn()> = {
        let created = Rc::clone(&created);
        Rc::new(move || match created.get() {
            Some(root) => println!("committed {}", root.name),
            None => println!("committed before the root is set"),
        })
    };
    let root = create_container(name, on_commit);
    commit(&root);
    let _ = created.set(Rc::clone(&root));
    root
}

// Set only where it's kept: `None` after, where it isn't.
fn kept_only(name: &str, keep: bool) -> Rc<Root> {
    let created: Rc<OnceCell<Rc<Root>>> = Rc::default();
    let on_commit: Rc<dyn Fn()> = {
        let created = Rc::clone(&created);
        Rc::new(move || match created.get() {
            Some(root) => println!("committed {}", root.name),
            None => println!("committed a root that isn't kept"),
        })
    };
    let root = create_container(name, on_commit);
    if keep {
        let _ = created.set(Rc::clone(&root));
    }
    root
}

// Set on each turn: the first root's, after.
fn each_turn(names: &[&str]) -> Vec<Rc<Root>> {
    let created: Rc<OnceCell<Rc<Root>>> = Rc::default();
    let mut roots = Vec::new();
    for name in names {
        let on_commit: Rc<dyn Fn()> = {
            let created = Rc::clone(&created);
            Rc::new(move || match created.get() {
                Some(root) => println!("committed, the first {}", root.name),
                None => println!("committed before the root is set"),
            })
        };
        let root = create_container(name, on_commit);
        let _ = created.set(Rc::clone(&root));
        roots.push(root);
    }
    roots
}

// Set to another root than the call's.
fn another(name: &str, other: &Rc<Root>) -> Rc<Root> {
    let created: Rc<OnceCell<Rc<Root>>> = Rc::default();
    let on_commit: Rc<dyn Fn()> = {
        let created = Rc::clone(&created);
        Rc::new(move || match created.get() {
            Some(root) => println!("committed, read as {}", root.name),
            None => println!("committed before the root is set"),
        })
    };
    let root = create_container(name, on_commit);
    let _ = created.set(Rc::clone(other));
    root
}

fn named(name: Option<String>) -> Rc<Option<String>> {
    Rc::new(name)
}

// What the `Rc` holds is an `Option`, `None` the cell's `Some` holds.
fn unnamed(name: Option<String>) -> Rc<dyn Fn()> {
    let created: Rc<OnceCell<Rc<Option<String>>>> = Rc::default();
    let report: Rc<dyn Fn()> = {
        let created = Rc::clone(&created);
        Rc::new(move || match created.get() {
            Some(name) => println!("set, named {name:?}"),
            None => println!("not set"),
        })
    };
    report();
    let name = named(name);
    let _ = created.set(Rc::clone(&name));
    report
}

struct Marker;

// What the `Rc` holds is a struct of no fields, `undefined`, which its
// `Some` boxes too.
fn marked() -> Rc<dyn Fn()> {
    let created: Rc<OnceCell<Rc<Marker>>> = Rc::default();
    let report: Rc<dyn Fn()> = {
        let created = Rc::clone(&created);
        Rc::new(move || println!("marked: {}", created.get().is_some()))
    };
    report();
    let marker = Rc::new(Marker);
    let _ = created.set(Rc::clone(&marker));
    report
}

fn main() {
    let a = committed_between("a");
    commit(&a);

    commit(&kept_only("b", true));
    commit(&kept_only("c", false));

    for root in each_turn(&["d", "e"]) {
        commit(&root);
    }

    let other = create_container("other", Rc::new(|| {}));
    commit(&another("f", &other));

    unnamed(None)();
    unnamed(Some("h".to_string()))();
    marked()();

}
