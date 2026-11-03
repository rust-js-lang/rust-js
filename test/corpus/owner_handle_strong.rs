// A callback made before the root it reads, as ReactDOMRootFB's legacy
// roots wrap the user's callback before `createHydrationContainer` makes
// the root it's given to: it closes over a cell set once the root is made,
// which holds the root's `Rc` itself, not a `Weak` (#577). The cell is the
// root's `let`, which `root = createContainer(..)` assigns and the callback
// reads, `undefined` until then as the cell's `None` is: one called before,
// by the call that makes the root or by its maker, reads `None`, and its
// `unwrap` panics, as Rust's does (ADR 0317, amended). Those the cell can't
// be the variable of are cells, as written (`owner_handle_strong_cells`).

use std::cell::{OnceCell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

type Callback = dyn Fn();

struct Root {
    name: String,
    callbacks: RefCell<Vec<Rc<Callback>>>,
}

// What a root calls before it's returned, before its caller can set the
// cell.
#[derive(Clone, Copy)]
enum Early {
    Nothing,
    Called,
}

fn create_container(name: &str, callback: Option<Rc<Callback>>, early: Early) -> Rc<Root> {
    let root = Rc::new(Root {
        name: name.to_string(),
        callbacks: RefCell::new(Vec::new()),
    });
    if let Some(callback) = callback {
        if let Early::Called = early {
            callback();
        }
        root.callbacks.borrow_mut().push(callback);
    }
    root
}

fn flush(root: &Root) {
    let callbacks = root.callbacks.borrow().clone();
    for callback in callbacks {
        callback();
    }
}

fn public_instance(root: &Rc<Root>) -> String {
    format!("the instance of {}", root.name)
}

fn legacy_root(name: &str, callback: Option<Rc<dyn Fn(String)>>, early: Early) -> Rc<Root> {
    let created: Rc<OnceCell<Rc<Root>>> = Rc::default();
    let mut wrapped: Option<Rc<Callback>> = None;
    if let Some(original_callback) = callback {
        let created = Rc::clone(&created);
        wrapped = Some(Rc::new(move || {
            let instance = public_instance(created.get().unwrap());
            original_callback(instance);
        }));
    }
    let root = create_container(name, wrapped, early);
    let _ = created.set(Rc::clone(&root));
    root
}

// Its maker calls it before the root is made: `None` there, as the `let`
// is `undefined`.
fn called_first(name: &str) -> Rc<Root> {
    let created: Rc<OnceCell<Rc<Root>>> = Rc::new(OnceCell::new());
    let report: Rc<Callback> = {
        let created = Rc::clone(&created);
        Rc::new(move || match created.get() {
            Some(root) => println!("{} is made", root.name),
            None => println!("no root yet"),
        })
    };
    report();
    let root = create_container(name, Some(Rc::clone(&report)), Early::Nothing);
    let _ = created.set(Rc::clone(&root));
    report();
    root
}

fn main() {
    let said = |instance: String| println!("called with {instance}");
    let a = legacy_root("a", Some(Rc::new(said)), Early::Nothing);
    flush(&a);
    flush(&a);

    let b = legacy_root("b", None, Early::Nothing);
    flush(&b);
    println!("{} has {} callbacks", b.name, b.callbacks.borrow().len());

    // Called by the call that makes the root, before the cell's set: its
    // `unwrap` panics.
    let early = catch_unwind(AssertUnwindSafe(|| legacy_root("c", Some(Rc::new(said)), Early::Called)));
    println!("c panicked: {}", early.is_err());

    let d = called_first("d");
    flush(&d);
}
