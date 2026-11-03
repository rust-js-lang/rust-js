// rd647 (B), the user's (1): a `#[rust_js::nullable]` field, not
// optional, of a struct only the crate makes is `null` or a value wherever
// it's read, so it's tested `=== null`, as upstream's `clones === null`: a
// plain `Option`, a `Cell`'s and a `RefCell`'s, read whole, through a
// `let`, a `borrow()` and `as_ref()`. A derived `Default` and an update's
// `..Default::default()` make each `null`, as a literal does. An optional
// one, absent until it's set, and one of a type parameter's value, which
// generic code stores as it's given, `undefined` too, stay `== null`.
// Native and JS agree.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
pub struct State {
    pub name: String,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub x: Option<u32>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub count: Cell<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub clones: RefCell<Option<Rc<RefCell<Vec<u32>>>>>,
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub label: Option<u32>,
}

pub struct Cursor<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current: RefCell<T>,
}

fn push<T>(cursor: &Cursor<T>, value: T) -> T {
    cursor.current.replace(value)
}

fn tests(state: &State) -> String {
    let mut out = String::new();
    if state.x.is_none() {
        out.push('x');
    }
    if state.label.is_none() {
        out.push('l');
    }
    match state.x {
        None => out.push('?'),
        Some(_) => out.push('!'),
    }
    let x = state.x;
    match x {
        None => out.push('-'),
        Some(x) => out.push_str(&x.to_string()),
    }
    if state.count.get().is_some() {
        out.push('c');
    }
    if state.clones.borrow().is_none() {
        out.push('n');
    }
    let clones = state.clones.borrow();
    match clones.as_ref() {
        None => out.push('0'),
        Some(clones) => out.push_str(&clones.borrow().len().to_string()),
    }
    out
}

fn main() {
    let made = State {
        name: "made".to_string(),
        x: Some(2),
        count: Cell::new(Some(3)),
        clones: RefCell::new(Some(Rc::new(RefCell::new(vec![1, 2])))),
        label: Some(7),
    };
    let defaulted = State::default();
    let updated = State {
        name: "updated".to_string(),
        x: Some(4),
        ..Default::default()
    };
    let none = State {
        name: "none".to_string(),
        x: None,
        count: Cell::new(None),
        clones: RefCell::new(None),
        label: None,
    };
    for state in [&made, &defaulted, &updated, &none] {
        println!("{} {}", state.name, tests(state));
    }
    made.count.set(None);
    made.clones.replace(None);
    println!("{}", tests(&made));
    let cursor = Cursor {
        current: RefCell::new(Some(1)),
    };
    let was = push(&cursor, None);
    println!("{was:?} {}", cursor.current.borrow().is_none());
}
