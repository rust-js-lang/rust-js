// An untagged enum's variant of a `RefCell` of an array is the array, and
// of an `Rc` what it points at (ADRs 0214, 0863): React's `ThenableState`,
// `[]` in production and `{ didWarnAboutUncachedPromise, thenables }` in
// DEV, told apart by `Array.isArray`.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct StateDev {
    warned: Cell<bool>,
    items: RefCell<Vec<f64>>,
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum State {
    Prod(RefCell<Vec<f64>>),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Dev(StateDev),
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Shared {
    Many(Rc<RefCell<Vec<f64>>>),
    One(f64),
}

fn items(state: &State) -> &RefCell<Vec<f64>> {
    match state {
        State::Dev(dev) => &dev.items,
        State::Prod(prod) => prod,
    }
}

fn create(dev: bool) -> State {
    if dev {
        State::Dev(StateDev {
            warned: Cell::new(false),
            items: RefCell::new(Vec::new()),
        })
    } else {
        State::Prod(RefCell::new(Vec::new()))
    }
}

fn track(state: &State, item: f64) -> usize {
    if let State::Dev(dev) = state {
        dev.warned.set(true);
    }
    items(state).borrow_mut().push(item);
    items(state).borrow().len()
}

fn total(shared: &Shared) -> f64 {
    match shared {
        Shared::Many(items) => items.borrow().iter().sum(),
        Shared::One(n) => *n,
    }
}

fn main() {
    for dev in [false, true] {
        let state = create(dev);
        track(&state, 1.0);
        let warned = matches!(&state, State::Dev(dev) if dev.warned.get());
        println!("{} {} {warned}", track(&state, 2.0), items(&state).borrow()[1]);
    }
    let shared = Shared::Many(Rc::new(RefCell::new(vec![1.0, 2.5])));
    if let Shared::Many(items) = &shared {
        items.borrow_mut().push(3.0);
    }
    println!("{} {}", total(&shared), total(&Shared::One(4.0)));
}
