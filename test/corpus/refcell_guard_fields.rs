// A `RefCell` guard's fields as locals of their own (#1815): while a
// `borrow_mut()`'s guard lives, nothing else reaches what its cell holds,
// and while a `borrow()`'s lives, nothing writes it, so a field tested
// through one `Deref` of the guard is known through the next, past any
// call: the undo stack's `index -= 1` under `index != 0` needs no wrap.
// Each turn of a loop knows nothing of the last turn's guard, which a call
// between them may have written; nor is a field that's a cell known,
// which another `Ref` may set; nor a guard's made into another by
// `RefMut::map`, given whole to a function, or lent through its
// `DerefMut` to a function or a method; nor what one borrow knew once its
// local holds another's. Each wrap and index is as Rust's.
use std::any::Any;
use std::cell::{Cell, RefCell, RefMut};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

struct History {
    entries: Vec<u32>,
    index: u32,
}

/// The undo stack's step back: its index tested, then stepped.
fn undo(history: &Rc<RefCell<History>>) -> Option<u32> {
    let mut history = history.borrow_mut();
    if history.index == 0 {
        return None;
    }
    history.index -= 1;
    Some(history.entries[history.index as usize])
}

/// A guard made each turn, a call between the turns setting its index.
fn stepped_back(history: &Rc<RefCell<History>>, between: &dyn Fn()) -> Vec<u32> {
    let mut seen = Vec::new();
    for turn in 0..2 {
        {
            let mut history = history.borrow_mut();
            if turn == 0 {
                history.index = 5;
            }
            history.index -= 1;
            seen.push(history.index);
        }
        between();
    }
    seen
}

/// Each turn's own test, of its own guard.
fn undone_each_turn(history: &Rc<RefCell<History>>, turns: u32) -> u32 {
    let mut undone = 0;
    for _ in 0..turns {
        let mut history = history.borrow_mut();
        if history.index > 0 {
            history.index -= 1;
            undone += 1;
        }
    }
    undone
}

struct Counted {
    count: Cell<u32>,
    label: u32,
}

/// A field that's a cell, which another `Ref` of it sets between.
fn counted_down(counted: &RefCell<Counted>, reset: &dyn Fn()) -> u32 {
    let counted = counted.borrow();
    if counted.count.get() == 0 {
        return 0;
    }
    reset();
    counted.count.get() - 1
}

/// A plain field of a `Ref`, which no call can write while it lives.
fn label_before(counted: &RefCell<Counted>, call: &dyn Fn()) -> u32 {
    let counted = counted.borrow();
    if counted.label == 0 {
        return 0;
    }
    call();
    counted.label - 1
}

/// A guard made into one of its field.
fn mapped_back(history: &Rc<RefCell<History>>) -> u32 {
    let mut index = RefMut::map(history.borrow_mut(), |history| &mut history.index);
    if *index == 0 {
        return 0;
    }
    *index -= 1;
    *index
}

/// A guard given whole to a function, which may set its index.
fn given_whole(history: &Rc<RefCell<History>>, step: fn(&mut RefMut<History>)) -> u32 {
    let mut history = history.borrow_mut();
    if history.index == 0 {
        return 0;
    }
    step(&mut history);
    history.index -= 1;
    history.index
}

/// A guard lent through its `DerefMut` to a function, which may set its
/// index.
fn lent_through(history: &Rc<RefCell<History>>, step: fn(&mut History)) -> u32 {
    let mut history = history.borrow_mut();
    if history.index == 0 {
        return 0;
    }
    step(&mut history);
    history.index -= 1;
    history.index
}

impl History {
    fn rewind(&mut self) {
        self.index = 0;
    }
}

/// A guard's `DerefMut` given to a method, which sets its index.
fn rewound_by_a_method(history: &Rc<RefCell<History>>) -> u32 {
    let mut history = history.borrow_mut();
    if history.index == 0 {
        return 0;
    }
    history.rewind();
    history.index -= 1;
    history.index
}

/// A guard's local made another cell's guard between the test and the
/// step.
fn borrowed_again(first: &RefCell<History>, second: &RefCell<History>) -> u32 {
    let mut history = first.borrow_mut();
    if history.index == 0 {
        return 0;
    }
    history = second.borrow_mut();
    history.index -= 1;
    history.index
}

fn main() {
    let history = Rc::new(RefCell::new(History {
        entries: vec![10, 11, 12],
        index: 2,
    }));
    check("undo", || undo(&history));
    check("undo again", || undo(&history));
    check("undo at the first", || undo(&history));
    history.borrow_mut().index = 7;
    check("undo past the end", || undo(&history));
    let reset = Rc::clone(&history);
    check("stepped back", || {
        stepped_back(&history, &|| reset.borrow_mut().index = 0)
    });
    history.borrow_mut().index = 2;
    check("undone each turn", || undone_each_turn(&history, 3));
    let counted = Rc::new(RefCell::new(Counted {
        count: Cell::new(1),
        label: 3,
    }));
    let other = Rc::clone(&counted);
    check("counted down", || {
        counted_down(&counted, &|| other.borrow().count.set(0))
    });
    check("label before", || {
        label_before(&counted, &|| other.borrow().count.set(5))
    });
    history.borrow_mut().index = 1;
    check("mapped back", || mapped_back(&history));
    check("mapped back at the first", || mapped_back(&history));
    history.borrow_mut().index = 1;
    check("given whole", || given_whole(&history, |history| history.index = 0));
    history.borrow_mut().index = 1;
    check("lent through", || lent_through(&history, |history| history.index = 0));
    history.borrow_mut().index = 1;
    check("rewound by a method", || rewound_by_a_method(&history));
    let second = RefCell::new(History {
        entries: Vec::new(),
        index: 0,
    });
    history.borrow_mut().index = 1;
    check("borrowed again", || borrowed_again(&history, &second));
}
