//@ library-differs
// A guard a `let` holds in generic code while nothing in its block can
// ask whether its cell is borrowed, beside another guard of the cell held
// while a callback runs, which is counted: the first is checked where
// it's made, and not counted, as a momentary one (#1815). The callback
// meets the counted one, `try_borrow_mut()` failing, as natively.
use std::cell::RefCell;
use std::rc::Rc;

struct Open {
    group: String,
    time: f64,
}

struct History<State> {
    entries: Vec<Rc<State>>,
    open: Option<Open>,
}

fn make<State>(history: Rc<RefCell<History<State>>>) -> (impl Fn(Option<String>) -> bool, impl Fn(&dyn Fn())) {
    let h1 = Rc::clone(&history);
    let one = move |group: Option<String>| {
        let history = h1.borrow_mut();
        let time = 3.0;
        group.as_ref().is_some_and(|group| {
            !group.is_empty()
                && history
                    .open
                    .as_ref()
                    .is_some_and(|open| &open.group == group && time - open.time < 5.0)
        })
    };
    let each = move |f: &dyn Fn()| {
        let _held = history.borrow();
        f();
    };
    (one, each)
}

fn main() {
    let history = Rc::new(RefCell::new(History::<i32> {
        entries: Vec::new(),
        open: Some(Open {
            group: "a".to_owned(),
            time: 1.0,
        }),
    }));
    let (one, each) = make(Rc::clone(&history));
    each(&|| println!("{}", history.try_borrow_mut().is_err()));
    println!("{} {}", one(Some("a".to_owned())), history.borrow().entries.len());
}
