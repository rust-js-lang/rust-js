// An `Option` combinator's subject, a field of a struct only the program
// makes, read in a closure through a `borrow_mut()` of an `Rc<RefCell<_>>`
// it captures, as the undo stack's `record` reads `history.open` (#1908's
// scope): no getter answers it, so where nothing runs between its test and
// its reads it's read again, `history.open != null && history.open.group ===
// group`, as upstream's `open?.group === group`; where the closure calls
// what may do anything between its test and a read, it's read once, into a
// `const`, as Rust reads it. A closure that writes the field it's given is
// of more than one expression, a function called with the field, read
// before the write, `map(s.x)`: what keeps the reads apart where they'd
// differ.
use std::cell::RefCell;
use std::rc::Rc;

struct Open {
    group: String,
    time: f64,
}

struct History {
    open: Option<Open>,
    asked: u32,
}

/// The undo stack's `record`: a closure that holds the history.
fn joiner(history: Rc<RefCell<History>>) -> Box<dyn Fn(Option<&str>, f64) -> bool> {
    Box::new(move |group, time| {
        let mut history = history.borrow_mut();
        history.asked += 1;
        let joined = group.is_some_and(|group| {
            !group.is_empty()
                && history
                    .open
                    .as_ref()
                    .is_some_and(|open| open.group == group && time - open.time < 500.0)
        });
        if !joined {
            history.open = None;
        }
        joined
    })
}

/// The same, a call between its test and a read.
fn checker(history: Rc<RefCell<History>>, check: Box<dyn Fn(f64) -> bool>) -> Box<dyn Fn(Option<&str>) -> bool> {
    Box::new(move |group| {
        let mut history = history.borrow_mut();
        history.asked += 1;
        let checked = group.is_some_and(|group| {
            !group.is_empty()
                && history
                    .open
                    .as_ref()
                    .is_some_and(|open| check(open.time) && open.group == group)
        });
        if !checked {
            history.open = None;
        }
        checked
    })
}

struct Slot {
    x: Option<i32>,
}

fn reset(x: &mut Option<i32>) -> i32 {
    *x = None;
    100
}

/// A closure that writes the field `map` reads, after it's read.
fn written_after() -> (Option<i32>, Option<i32>) {
    let mut slot = Slot { x: Some(1) };
    let r = slot.x.map(|v| {
        slot.x = None;
        v + 1
    });
    (r, slot.x)
}

/// One that writes it through a `&mut` of it, in one expression.
fn reset_after() -> (Option<i32>, Option<i32>) {
    let mut slot = Slot { x: Some(1) };
    let r = slot.x.map(|v| reset(&mut slot.x) + v);
    (r, slot.x)
}

fn opened(history: &Rc<RefCell<History>>, group: &str) {
    history.borrow_mut().open = Some(Open {
        group: group.to_string(),
        time: 0.0,
    });
}

fn main() {
    let history = Rc::new(RefCell::new(History { open: None, asked: 0 }));
    let joins = joiner(Rc::clone(&history));
    println!("{}", joins(Some("insert"), 100.0));
    opened(&history, "insert");
    println!("{}", joins(Some("insert"), 100.0));
    println!("{}", joins(Some("insert"), 900.0));
    opened(&history, "insert");
    println!("{}", joins(None, 100.0));
    opened(&history, "insert");
    let checks = checker(
        Rc::clone(&history),
        Box::new(|time| {
            println!("checked {time}");
            time < 1.0
        }),
    );
    println!("{}", checks(Some("insert")));
    println!("{}", checks(Some("other")));
    println!("asked {}", history.borrow().asked);
    println!("{:?} {:?}", written_after(), reset_after());
}
