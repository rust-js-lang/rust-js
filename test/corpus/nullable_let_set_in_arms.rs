// rd647 (C): a late `let` each arm writes, `previous_measurements = None;`,
// `= Some(..collect())` and `= memoized.take()`, React's
// measureUpdateViewTransition's `previousMeasurements`, read by the crate's
// own nullable parameter through `as_deref()` and by `map_or`, is `null`
// for `None` where it's written, as rd632's (B) makes a `let` of one value
// so: each arm's write counted once, a `Some` an arm moves in among them.
// Native and JS agree.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct State {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub clones: RefCell<Option<Rc<RefCell<Vec<u32>>>>>,
}

#[cfg_attr(rust_js, rust_js::nullable(previous))]
fn measured(previous: Option<&[u32]>) -> usize {
    previous.map_or(0, |previous| previous.len())
}

fn measure(x: u32) -> u32 {
    x * 2
}

fn update(state: &State, gesture: bool, memoized: &Cell<Option<Vec<u32>>>) -> usize {
    let previous_measurements: Option<Vec<u32>>;
    if gesture {
        let clones = state.clones.borrow();
        match clones.as_ref() {
            None => {
                previous_measurements = None;
            }
            Some(clones) => {
                previous_measurements = Some(clones.borrow().iter().copied().map(measure).collect());
            }
        }
    } else {
        previous_measurements = memoized.take();
    }
    let found = measured(previous_measurements.as_deref());
    found + previous_measurements.map_or(0, |previous| previous.len())
}

fn main() {
    let none = State {
        clones: RefCell::new(None),
    };
    let some = State {
        clones: RefCell::new(Some(Rc::new(RefCell::new(vec![1, 2, 3])))),
    };
    let memoized = Cell::new(Some(vec![4]));
    println!("{} {}", update(&none, true, &memoized), update(&some, true, &memoized));
    println!("{} {}", update(&none, false, &memoized), update(&some, false, &memoized));
}
