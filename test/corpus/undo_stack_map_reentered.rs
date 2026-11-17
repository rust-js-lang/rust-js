//@ library-differs
// #1815's undo stack, as its port writes it, with a fixed time for
// `Date.now()` and its settings' enum tagged: its history's index isn't always below its entries'
// length. `map` gives its borrow back before it calls `update`, which may
// use the stack, and a `record` there pushes an entry; `map` then writes
// the shorter list back, past which `redo` indexes beyond it, and panics
// as Rust's does, where an unchecked index would read `undefined`. So
// `redo`'s index and the wraps of `index` stay checked.
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

/// How an undo stack keeps entries. Settings can change after creation.
#[derive(Default)]
pub struct UndoStackOptions {
    /// How many entries are kept. Defaults to 100.
    pub limit: Cell<Option<f64>>,
    /// Milliseconds of pause after which typing starts a new entry. Defaults to 500.
    pub group_delay: Cell<Option<f64>>,
}

pub enum UndoStackSettings<'a> {
    Options(Rc<UndoStackOptions>),
    Getter(Box<dyn Fn() -> Option<Rc<UndoStackOptions>> + 'a>),
}

#[derive(Default)]
pub struct UndoRecordOptions {
    pub group: Option<String>,
    pub time: Option<f64>,
}

struct OpenGroup {
    group: String,
    time: f64,
}

struct History<State> {
    entries: Vec<Rc<State>>,
    index: usize,
    open: Option<OpenGroup>,
}

/// An undo stack for any state. Each function retains its stack when called alone.
pub struct UndoStack<'a, State> {
    /// Records an edit. The same group within group_delay joins the last entry.
    pub record: Box<dyn Fn(Rc<State>, Option<&UndoRecordOptions>) + 'a>,
    /// Ends the open group, like after a caret jump.
    pub seal: Box<dyn Fn() + 'a>,
    /// Steps back, or returns None at the first entry.
    pub undo: Box<dyn Fn() -> Option<Rc<State>> + 'a>,
    /// Steps forward, or returns None at the last entry.
    pub redo: Box<dyn Fn() -> Option<Rc<State>> + 'a>,
    pub can_undo: Box<dyn Fn() -> bool + 'a>,
    pub can_redo: Box<dyn Fn() -> bool + 'a>,
    /// Rewrites every entry without creating a new history entry.
    pub map: Box<dyn Fn(&dyn Fn(&Rc<State>) -> Rc<State>) + 'a>,
    /// Starts over, like after a controlled value from outside.
    pub reset: Box<dyn Fn(Rc<State>) + 'a>,
}

/// State snapshots are shared: undo and redo return the exact recorded state.
/// A settings getter is read on every edit, so later changes apply.
pub fn create_undo_stack<'a, State: 'a>(
    initial: Rc<State>,
    options: Option<UndoStackSettings<'a>>,
) -> UndoStack<'a, State> {
    let history = Rc::new(RefCell::new(History {
        entries: vec![initial],
        index: 0,
        open: None,
    }));
    let record = {
        let history = Rc::clone(&history);
        Box::new(move |state, options_for_record: Option<&UndoRecordOptions>| {
            let defaults = UndoRecordOptions::default();
            let options_for_record = options_for_record.unwrap_or(&defaults);
            let group = options_for_record.group.clone();
            let time = options_for_record.time.unwrap_or(0.0);
            let settings = match &options {
                Some(UndoStackSettings::Options(options)) => Some(Rc::clone(options)),
                Some(UndoStackSettings::Getter(get)) => get(),
                None => None,
            };
            let limit = settings
                .as_ref()
                .and_then(|settings| settings.limit.get())
                .unwrap_or(100.0);
            let group_delay = settings
                .as_ref()
                .and_then(|settings| settings.group_delay.get())
                .unwrap_or(500.0);
            let mut history = history.borrow_mut();
            let end = history.index + 1;
            history.entries.truncate(end);
            let joins = group.as_ref().is_some_and(|group| {
                !group.is_empty()
                    && history
                        .open
                        .as_ref()
                        .is_some_and(|open| &open.group == group && time - open.time < group_delay)
            });
            if joins {
                let index = history.index;
                history.entries[index] = state;
            } else {
                history.entries.push(state);
                if history.entries.len() as f64 > limit + 1.0 {
                    history.entries.remove(0);
                }
                history.index = history.entries.len() - 1;
            }
            history.open = group
                .filter(|group| !group.is_empty())
                .map(|group| OpenGroup { group, time });
        })
    };
    let seal = {
        let history = Rc::clone(&history);
        Box::new(move || history.borrow_mut().open = None)
    };
    let undo = {
        let history = Rc::clone(&history);
        Box::new(move || {
            let mut history = history.borrow_mut();
            history.open = None;
            if history.index == 0 {
                return None;
            }
            history.index -= 1;
            Some(Rc::clone(&history.entries[history.index]))
        })
    };
    let redo = {
        let history = Rc::clone(&history);
        Box::new(move || {
            let mut history = history.borrow_mut();
            history.open = None;
            if history.index == history.entries.len() - 1 {
                return None;
            }
            history.index += 1;
            Some(Rc::clone(&history.entries[history.index]))
        })
    };
    let can_undo = {
        let history = Rc::clone(&history);
        Box::new(move || history.borrow().index > 0)
    };
    let can_redo = {
        let history = Rc::clone(&history);
        Box::new(move || {
            let history = history.borrow();
            history.index < history.entries.len() - 1
        })
    };
    let map = {
        let history = Rc::clone(&history);
        Box::new(move |update: &dyn Fn(&Rc<State>) -> Rc<State>| {
            // Release the borrow before user callbacks: they may re-enter the stack.
            let entries = history.borrow().entries.clone();
            let entries = entries.iter().map(update).collect();
            history.borrow_mut().entries = entries;
        })
    };
    let reset = Box::new(move |state| {
        let mut history = history.borrow_mut();
        history.entries = vec![state];
        history.index = 0;
        history.open = None;
    });
    UndoStack {
        record,
        seal,
        undo,
        redo,
        can_undo,
        can_redo,
        map,
        reset,
    }
}

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

fn edit(stack: &UndoStack<i32>, value: i32, group: &str, time: f64) {
    (stack.record)(
        Rc::new(value),
        Some(&UndoRecordOptions {
            group: Some(group.to_owned()),
            time: Some(time),
        }),
    );
}

fn main() {
    let options = Rc::new(UndoStackOptions::default());
    options.limit.set(Some(2.0));
    let stack = create_undo_stack(Rc::new(0), Some(UndoStackSettings::Options(options)));
    edit(&stack, 1, "insert", 0.0);
    edit(&stack, 2, "insert", 100.0);
    (stack.seal)();
    edit(&stack, 3, "insert", 200.0);
    edit(&stack, 4, "other", 300.0);
    check("undo", || (stack.undo)().map(|state| *state));
    check("can redo", || (stack.can_redo)());
    check("redo", || (stack.redo)().map(|state| *state));
    let stack = create_undo_stack(Rc::new(0), Some(UndoStackSettings::Getter(Box::new(|| None))));
    (stack.map)(&|state| {
        (stack.record)(Rc::new(1), None);
        Rc::new(**state + 10)
    });
    check("can undo", || (stack.can_undo)());
    check("can redo past the end", || (stack.can_redo)());
    check("redo past the end", || (stack.redo)().map(|state| *state));
    check("redo again", || (stack.redo)().map(|state| *state));
    check("undo back", || (stack.undo)().map(|state| *state));
    (stack.reset)(Rc::new(5));
    check("undo after reset", || (stack.undo)().map(|state| *state));
}
