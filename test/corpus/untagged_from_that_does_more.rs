// A `From` into an untagged enum is its body, called, wherever it does
// more than make the variant of its argument (#1775, ADR 0214, amended):
// it may own what it's given, convert it, count its calls, or choose a
// variant. Only one that's the variant of its argument is the value
// itself, `From<String>`'s here; `into()`, `From::from`, a function
// value, generic code's `T: Into<Detail>` and `W: From<T>`, and `?` each
// run the same `from`.

use std::cell::{Cell, RefCell};

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, PartialEq)]
pub enum Detail {
    Text(String),
    Number(u32),
}

impl From<&str> for Detail {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

thread_local! {
    static MADE: Cell<u32> = const { Cell::new(0) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

impl From<u32> for Detail {
    fn from(value: u32) -> Self {
        MADE.with(|made| made.set(made.get() + 1));
        Self::Number(value + 2)
    }
}

impl From<String> for Detail {
    fn from(value: String) -> Self {
        Detail::Text(value)
    }
}

// One that chooses its variant by what it's given.
impl From<i64> for Detail {
    fn from(value: i64) -> Self {
        LOG.with(|log| log.borrow_mut().push(format!("from {value}")));
        if value < 0 {
            Detail::Text(format!("minus {}", -value))
        } else {
            Detail::Number(value as u32)
        }
    }
}

fn detail<T: Into<Detail>>(value: T) -> Detail {
    value.into()
}

fn made<W: From<u32>>(value: u32) -> W {
    W::from(value)
}

fn show(detail: &Detail) -> String {
    match detail {
        Detail::Text(text) => text.clone(),
        Detail::Number(n) => n.to_string(),
    }
}

fn checked(n: i64) -> Result<u32, Detail> {
    if n > 100 {
        Err(n)?;
    }
    if n == 7 {
        Err("seven")?;
    }
    Ok(n as u32)
}

fn logged(what: &str) -> i64 {
    LOG.with(|log| log.borrow_mut().push(what.to_owned()));
    what.len() as i64
}

fn main() {
    let text = detail("hello");
    let three: Detail = 3u32.into();
    let five = Detail::from(5u32);
    println!(
        "{}:{}:{}:{}",
        show(&text),
        show(&three),
        show(&five),
        MADE.with(Cell::get)
    );

    let owned: Detail = String::from("kept").into();
    let parts: Vec<Detail> = [1u32, 2].into_iter().map(Detail::from).collect();
    let words: Vec<Detail> = ["a", "b"].into_iter().map(Into::into).collect();
    let make: fn(u32) -> Detail = Detail::from;
    let generic: Detail = made(20);
    println!(
        "{} {} {} {} {} {} {} {}",
        show(&owned),
        show(&parts[0]),
        show(&parts[1]),
        show(&words[0]),
        show(&words[1]),
        show(&make(10)),
        show(&detail(String::from("given"))),
        show(&generic)
    );
    println!("made {}", MADE.with(Cell::get));

    // Each `from` runs where it's called, in order with what's around it.
    let chosen = vec![
        Detail::from(logged("one")),
        Detail::from(-4i64),
        detail(logged("three")),
    ];
    println!("{chosen:?}");
    println!("{:?} {:?} {:?}", checked(3), checked(7), checked(150));
    LOG.with(|log| println!("{:?}", log.borrow()));
    // One given what a borrow of the cell it writes read, once it ends.
    let held = LOG.with(|log| log.borrow().len());
    let after: Detail = LOG.with(|log| {
        let first = log.borrow()[0].clone();
        Detail::from(first.len() as i64)
    });
    println!("{held} {after:?} {}", LOG.with(|log| log.borrow().len()));
    println!("{}", Detail::from("x") == Detail::Text("x".to_owned()));
}
