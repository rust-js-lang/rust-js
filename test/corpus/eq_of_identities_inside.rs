// `==` of what holds a type equal only to itself, as React's fibers are
// (ADR 0864): an `Option` of a reference to an `Option` of one, nested
// `Option`s, a tuple, a `Vec`, a derived struct and an untagged enum of
// one, each compares it by itself, `===`, as its own `eq` does, never by
// what's in it, which `$eq` walks: two fibers alike are told apart, and a
// cycle of them is never walked (ADR 1015).
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

struct Fiber {
    tag: u32,
    alternate: RefCell<Option<Rc<Fiber>>>,
}

impl PartialEq for Fiber {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Fiber {}

impl Hash for Fiber {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

fn fiber(tag: u32) -> Rc<Fiber> {
    Rc::new(Fiber {
        tag,
        alternate: RefCell::new(None),
    })
}

#[derive(PartialEq)]
struct Listener {
    instance: Option<Rc<Fiber>>,
    name: &'static str,
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(PartialEq)]
enum Target {
    Fiber(Rc<Fiber>),
    Index(u32),
}

#[derive(PartialEq)]
struct Pointer {
    target: Target,
}

fn same_instance(previous: Option<&Option<Rc<Fiber>>>, instance: &Option<Rc<Fiber>>) -> bool {
    previous == Some(instance)
}

fn same_nested(a: &Option<Option<Rc<Fiber>>>, b: &Option<Option<Rc<Fiber>>>) -> bool {
    a == b
}

fn same_pair(a: &(u32, Rc<Fiber>), b: &(u32, Rc<Fiber>)) -> bool {
    a == b
}

fn same_items(a: &[Option<Rc<Fiber>>], b: &[Option<Rc<Fiber>>]) -> bool {
    a == b
}

fn same_listener(a: &Listener, b: &Listener) -> bool {
    a == b
}

fn same_pointer(a: &Pointer, b: &Pointer) -> bool {
    a == b
}

fn main() {
    // Two alike, each the other's alternate: a cycle.
    let current = fiber(1);
    let work = fiber(1);
    *current.alternate.borrow_mut() = Some(Rc::clone(&work));
    *work.alternate.borrow_mut() = Some(Rc::clone(&current));
    let (a, b) = (Some(Rc::clone(&current)), Some(Rc::clone(&work)));
    println!(
        "{} {} {} {}",
        same_instance(Some(&a), &a),
        same_instance(Some(&a), &b),
        same_instance(None, &None),
        same_instance(Some(&None), &None)
    );
    println!(
        "{} {} {}",
        same_nested(&Some(a.clone()), &Some(a.clone())),
        same_nested(&Some(a.clone()), &Some(b.clone())),
        same_nested(&Some(None), &None)
    );
    println!(
        "{} {}",
        same_pair(&(1, Rc::clone(&current)), &(1, Rc::clone(&current))),
        same_pair(&(1, Rc::clone(&current)), &(1, Rc::clone(&work)))
    );
    println!(
        "{} {}",
        same_items(&[a.clone(), None], &[a.clone(), None]),
        same_items(&[a.clone()], &[b.clone()])
    );
    let listener = |instance: &Option<Rc<Fiber>>| Listener {
        instance: instance.clone(),
        name: "click",
    };
    println!(
        "{} {}",
        same_listener(&listener(&a), &listener(&a)),
        same_listener(&listener(&a), &listener(&b))
    );
    let at = |fiber: &Rc<Fiber>| Pointer {
        target: Target::Fiber(Rc::clone(fiber)),
    };
    let index = Pointer {
        target: Target::Index(2),
    };
    println!(
        "{} {} {}",
        same_pointer(&at(&current), &at(&current)),
        same_pointer(&at(&current), &at(&work)),
        same_pointer(&index, &at(&work))
    );
    *current.alternate.borrow_mut() = None;
    println!("{}", current.tag + work.tag);
}
