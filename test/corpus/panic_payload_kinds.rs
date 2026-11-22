// A panic's payload is a `&'static str` exactly where its
// `fmt::Arguments::as_str()` is `Some`, and a `String` otherwise, as Rust's
// panic handler makes it (ADR 0660, amended): `panic!("plain")`, `unwrap()`,
// an `assert!` and a division by zero give a `&str`;
// `panic!("n = {n}")`, `panic!("{}", n)`, `expect`, an index out of bounds, a slice's or a
// string's range, a `RefCell` borrowed twice and an `assert_eq!` give a
// `String`. Resumed, a payload keeps its kind.
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

struct Noisy;

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop noisy");
    }
}

type Run = fn(Box<dyn Fn()>);

thread_local! {
    static RUN: Cell<Option<Run>> = const { Cell::new(None) };
}

// An `expect` whose panic drops what it was given as it unwinds.
fn run_given(task: Box<dyn Fn()>) {
    RUN.get().expect("no run given")(task)
}

fn kind(payload: &(dyn Any + Send)) -> String {
    let string = payload.downcast_ref::<String>();
    let str = payload.downcast_ref::<&str>();
    match (string, str) {
        (Some(message), None) => format!("String {message:?}"),
        (None, Some(message)) => format!("&str {message:?}"),
        (Some(_), Some(_)) => "both".to_string(),
        (None, None) => "neither".to_string(),
    }
}

fn shown(name: &str, f: impl FnOnce() + std::panic::UnwindSafe) {
    match catch_unwind(f) {
        Ok(()) => println!("{name}: no panic"),
        Err(payload) => println!("{name}: {}", kind(&*payload)),
    }
}

fn main() {
    let n = std::hint::black_box(4usize);
    let none: Option<u8> = std::hint::black_box(None);
    let v = std::hint::black_box(vec![1, 2, 3]);
    let text = std::hint::black_box(String::from("héllo"));
    shown("plain", || panic!("plain"));
    shown("empty", || panic!());
    shown("formatted", || panic!("n = {n}"));
    shown("display", || panic!("{}", n));
    shown("unwrap", || {
        none.unwrap();
    });
    shown("expect", || {
        none.expect("gone");
    });
    shown("index", || {
        let _ = v[n];
    });
    shown("array index", || {
        let a = [1, 2, 3];
        let _ = a[n];
    });
    shown("expect dropping", || {
        let noisy = Noisy;
        run_given(Box::new(move || {
            let _ = &noisy;
        }));
    });
    shown("slice", || {
        let _ = &v[..n];
    });
    shown("str", || {
        let _ = &text[..2];
    });
    shown("unreachable", || unreachable!());
    shown("unreachable message", || unreachable!("x"));
    shown("todo", || todo!());
    shown("assert", || assert!(n == 5));
    shown("assert message", || assert!(n == 5, "msg"));
    shown("assert formatted", || assert!(n == 5, "n is {n}"));
    shown("assert_eq", || assert_eq!(n, 5));
    shown("divide", || {
        let _ = std::hint::black_box(1) / std::hint::black_box(0);
    });
    let cell = RefCell::new(1);
    shown(
        "refcell",
        AssertUnwindSafe(|| {
            let _a = cell.borrow_mut();
            let _b = cell.borrow_mut();
        }),
    );
    shown("resumed str", || {
        let payload = catch_unwind(|| panic!("plain")).unwrap_err();
        resume_unwind(payload);
    });
    shown("resumed String", || {
        let payload = catch_unwind(|| panic!("n = {n}")).unwrap_err();
        resume_unwind(payload);
    });
    shown("resumed new String", || resume_unwind(Box::new(String::from("made"))));
    shown("resumed new str", || resume_unwind(Box::new("made")));
}
