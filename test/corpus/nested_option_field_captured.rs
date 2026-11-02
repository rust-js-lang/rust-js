// A closure's capture of a `#[rust_js::nullable]` `Option<Option<T>>`
// field, WebIDL's optional nullable member, is given held as the field
// holds it, its `None` absent, its `Some(None)` `null`, and read so in the
// closure's body: `|| f(entry.value.flatten())` reads `entry.value`, as
// React's Flight reply client's `catch_unwind` of `stringify` does (rd631).
// Borrowed and moved, by `Fn`, `FnMut` and `FnOnce`, read whole, flattened,
// matched, shown and given to a generic function, and of a maker that
// boxes it, against native Rust.
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Default)]
pub struct Entry {
    pub done: Option<bool>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub value: Option<Option<u32>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub name: Option<Option<String>>,
}

fn shown<T: std::fmt::Debug>(value: T) -> String {
    format!("{value:?}")
}

fn described(value: Option<u32>) -> String {
    match value {
        Some(v) => format!("v{v}"),
        None => "nothing".to_string(),
    }
}

fn caught(entry: &Entry) -> String {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if entry.value == Some(Some(13)) {
            panic!("unlucky");
        }
        described(entry.value.flatten())
    }));
    result.unwrap_or_else(|_| "caught".to_string())
}

fn borrowed(entry: Entry) -> String {
    let read = || described(entry.value.flatten());
    let kind = || match entry.value {
        None => "missing",
        Some(None) => "null",
        Some(Some(_)) => "value",
    };
    let whole = || shown(entry.value);
    let name = || entry.name.clone().flatten().unwrap_or_default();
    format!("{} {} {} {} {:?}", read(), kind(), whole(), name(), entry.done)
}

fn moved(entry: Entry) -> Box<dyn Fn() -> String> {
    let value = entry.value;
    Box::new(move || format!("{} {}", described(value.flatten()), value.is_none()))
}

fn once(entry: Entry) -> String {
    let take = move || entry.name.flatten().unwrap_or_else(|| "none".to_string());
    take()
}

fn counted(entry: &mut Entry) -> u32 {
    let mut value = entry.value;
    let mut bump = || {
        value = match value {
            None => Some(None),
            Some(None) => Some(Some(1)),
            Some(Some(v)) => Some(Some(v + 1)),
        };
        value.flatten().unwrap_or(0)
    };
    let first = bump();
    let second = bump();
    entry.value = value;
    first * 10 + second
}

// The field moved into a closure that sets it again.
fn rewritten(mut entry: Entry) -> String {
    let mut step = move || {
        entry.value = match entry.value {
            None => Some(None),
            Some(None) => Some(Some(7)),
            Some(Some(v)) => Some(Some(v * 2)),
        };
        format!("{} {}", shown(entry.value), described(entry.value.flatten()))
    };
    format!("{} / {}", step(), step())
}

// The field borrowed, then given whole to code that boxes it, then
// captured: the maker's box, held again for the closure.
fn boxed_first(entry: &Entry) -> String {
    let value = &entry.value;
    let before = shown(value);
    let read = || described(value.flatten());
    format!("{before} {}", read())
}

fn main() {
    let made = |at: u32| Entry {
        done: Some(at == 0),
        value: match at {
            0 => None,
            1 => Some(None),
            _ => Some(Some(at)),
        },
        name: match at {
            0 => None,
            1 => Some(None),
            _ => Some(Some(format!("n{at}"))),
        },
    };
    for at in [0, 1, 2, 13] {
        println!("{}", caught(&made(at)));
        println!("{}", borrowed(made(at)));
        println!("{}", moved(made(at))());
        println!("{}", once(made(at)));
        let mut entry = made(at);
        println!("{} {:?}", counted(&mut entry), entry.value);
        println!("{}", boxed_first(&made(at)));
        println!("{}", rewritten(made(at)));
    }
}
