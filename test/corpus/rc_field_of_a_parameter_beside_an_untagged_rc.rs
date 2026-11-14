//@ library-refused: JS can't tell this variant's `std::rc::Rc<Options>`
// A program's struct whose field holds an `Rc` of its type parameter,
// `History<State>`'s `Vec<Rc<State>>`, beside an untagged enum one of whose
// variants is an `Rc` of a struct, and an `Rc<i32>` compared by
// `Rc::ptr_eq`, which counts it (#1816): the field's `Rc<State>` is what
// the program's own bodies make, `State` the `i32` its caller gives, so the
// settings' `Rc<Options>` isn't counted and the enum's variants are told
// apart. An undo stack's, as a port of vercel-ui's keeps it, its tests'
// `Rc::ptr_eq` of the state it gives back. As a library, `Stack<State>`
// is a type a consumer can make, of any `State`, so its fields' `Rc<State>`
// may be any `Rc`, the `Rc<i32>` counted among them: refused there (ADR
// 0852's wildcard). A private one, `History`'s, is the library's own.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
pub struct Options {
    pub limit: Cell<Option<f64>>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Settings {
    Options(Rc<Options>),
    Getter(Box<dyn Fn() -> Option<Rc<Options>>>),
}

struct History<State> {
    entries: Vec<Rc<State>>,
    index: usize,
}

pub struct Stack<State> {
    pub record: Box<dyn Fn(Rc<State>)>,
    pub undo: Box<dyn Fn() -> Option<Rc<State>>>,
}

pub fn stack<State: 'static>(initial: Rc<State>, settings: Option<Settings>) -> Stack<State> {
    let history = Rc::new(RefCell::new(History {
        entries: vec![initial],
        index: 0,
    }));
    let record = {
        let history = Rc::clone(&history);
        Box::new(move |state| {
            let options = match &settings {
                Some(Settings::Options(options)) => Some(Rc::clone(options)),
                Some(Settings::Getter(get)) => get(),
                None => None,
            };
            let limit = options.and_then(|options| options.limit.get()).unwrap_or(100.0);
            let mut history = history.borrow_mut();
            let end = history.index + 1;
            history.entries.truncate(end);
            history.entries.push(state);
            if history.entries.len() as f64 > limit + 1.0 {
                history.entries.remove(0);
            }
            history.index = history.entries.len() - 1;
        })
    };
    let undo = Box::new(move || {
        let mut history = history.borrow_mut();
        if history.index == 0 {
            return None;
        }
        history.index -= 1;
        Some(Rc::clone(&history.entries[history.index]))
    });
    Stack { record, undo }
}

fn main() {
    let initial = Rc::new(0);
    let plain = stack(Rc::clone(&initial), None);
    (plain.record)(Rc::new(1));
    let back = (plain.undo)().unwrap();
    println!("{} {}", Rc::ptr_eq(&back, &initial), Rc::ptr_eq(&back, &Rc::new(0)));
    println!("{:?}", (plain.undo)());

    let options = Rc::new(Options::default());
    options.limit.set(Some(1.0));
    let limited = stack(Rc::new(10), Some(Settings::Options(Rc::clone(&options))));
    for n in 11..14 {
        (limited.record)(Rc::new(n));
    }
    let undone: Vec<Option<i32>> = (0..3).map(|_| (limited.undo)().map(|n| *n)).collect();
    println!("{undone:?}");

    let got = Rc::new(Options::default());
    got.limit.set(Some(2.0));
    let getter = stack(
        Rc::new(20),
        Some(Settings::Getter(Box::new(move || Some(Rc::clone(&got))))),
    );
    for n in 21..25 {
        (getter.record)(Rc::new(n));
    }
    let undone: Vec<Option<i32>> = (0..4).map(|_| (getter.undo)().map(|n| *n)).collect();
    println!("{undone:?}");
}
