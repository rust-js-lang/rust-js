// A struct literal of a struct with a `Cell` beside its plain fields is one
// expression: a plain field read through `&A`, `a.id`, can't change while a
// call made after it runs, so the call's value is made in the literal
// (#1045).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Instance {
    id: u32,
    parent: Cell<u32>,
    children: Rc<RefCell<Vec<u32>>>,
    text: RefCell<Option<String>>,
    context: u32,
}

fn should_set(t: &str) -> bool {
    t.len() > 3
}

fn compute(s: &str, c: u32) -> String {
    format!("{s}{c}")
}

fn clone_instance(instance: &Instance, t: &str, keep: bool, children: Option<Rc<RefCell<Vec<u32>>>>) -> Instance {
    Instance {
        id: instance.id,
        parent: Cell::new(instance.parent.get()),
        children: if keep { Rc::clone(&instance.children) } else { children.unwrap_or_default() },
        text: RefCell::new(if should_set(t) { Some(compute(t, instance.context)) } else { None }),
        context: instance.context,
    }
}

// A call that sets the cell it's made after: the cell's read stays first.
fn bumped(instance: &Instance) -> u32 {
    instance.parent.set(instance.parent.get() + 100);
    instance.parent.get()
}

struct Pair {
    id: u32,
    parent: Cell<u32>,
    after: u32,
}

fn pair(instance: &Instance) -> Pair {
    Pair { id: instance.id, parent: Cell::new(instance.parent.get()), after: bumped(instance) }
}

fn main() {
    let first = Instance {
        id: 1,
        parent: Cell::new(7),
        children: Rc::new(RefCell::new(vec![1, 2])),
        text: RefCell::new(None),
        context: 9,
    };
    let kept = clone_instance(&first, "text", true, None);
    let fresh = clone_instance(&first, "no", false, Some(Rc::new(RefCell::new(vec![3]))));
    println!("{} {} {:?} {:?}", kept.id, kept.parent.get(), kept.children.borrow(), kept.text.borrow());
    println!("{} {} {:?} {:?}", fresh.id, fresh.parent.get(), fresh.children.borrow(), fresh.text.borrow());
    let made = pair(&first);
    println!("{} {} {} {}", made.id, made.parent.get(), made.after, first.parent.get());
}
