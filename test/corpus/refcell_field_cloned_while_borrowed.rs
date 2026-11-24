// A derived clone of a struct or a variant whose `RefCell` field holds its
// value borrows the field while it clones what it holds, as std's
// `RefCell::clone` does: cloned while the field is mutably borrowed, it
// panics, even where the clone is never used; while it's shared, it clones
// (#2479).
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Debug)]
struct Holder {
    c: RefCell<u32>,
    name: String,
}

#[derive(Clone, Debug)]
enum Slot {
    Empty,
    Full { c: RefCell<u32> },
}

fn said(r: std::thread::Result<()>) -> String {
    match r {
        Ok(()) => "ok".to_string(),
        Err(payload) => match payload.downcast_ref::<&str>() {
            Some(text) => format!("panicked: {text}"),
            None => format!(
                "panicked: {}",
                payload.downcast_ref::<String>().map_or("?", |s| s.as_str())
            ),
        },
    }
}

fn main() {
    let h = Holder {
        c: RefCell::new(1),
        name: String::from("h"),
    };
    println!("{:?}", h.clone());
    let writing = h.c.borrow_mut();
    println!(
        "{}",
        said(catch_unwind(AssertUnwindSafe(|| println!("{:?}", h.clone()))))
    );
    println!(
        "{}",
        said(catch_unwind(AssertUnwindSafe(|| {
            let _ = h.clone();
        })))
    );
    drop(writing);
    let reading = h.c.borrow();
    println!("{:?} {}", h.clone(), *reading);
    drop(reading);

    let s = Slot::Full { c: RefCell::new(2) };
    let Slot::Full { c } = &s else { return };
    let writing = c.borrow_mut();
    println!(
        "{}",
        said(catch_unwind(AssertUnwindSafe(|| println!("{:?}", s.clone()))))
    );
    drop(writing);
    println!("{:?} {:?}", s.clone(), Slot::Empty.clone());
}
