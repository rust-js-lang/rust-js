// ADR 0861, amended (#665): a nullable thread-local stores `null` for
// `None`, `x ?? null` of what may be `undefined`, only where a read of it
// can tell them apart. One private to its module that's only tested, its
// `Some`'s value read, unwrapped or given a fallback is stored as it's
// given, `undefined` too: each read gives what native Rust does either
// way. One whose read is kept in a nullable field, a struct only the
// crate makes, which is tested `=== null`, or returned by a nullable
// function, or compared, keeps `?? null`: there a `None` stored
// `undefined` would read as a `Some`.
use std::cell::{Cell, RefCell};

struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    name: Option<&'static str>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static TESTED: Cell<Option<&'static str>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static CLONED: RefCell<Option<String>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COMBINED: Cell<Option<&'static str>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static HELD: Cell<Option<&'static str>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static RETURNED: Cell<Option<&'static str>> = const { Cell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static COMPARED: Cell<Option<&'static str>> = const { Cell::new(None) };
}

#[cfg_attr(rust_js, rust_js::nullable(name))]
fn set_tested(name: Option<&'static str>) {
    TESTED.set(name);
}

fn tested() -> String {
    let Some(name) = TESTED.get() else {
        return "none".to_owned();
    };
    let shown = TESTED.get().unwrap_or("-");
    let kept = TESTED.get().is_some_and(|n| n.len() == 1);
    format!("{name} {shown} {kept} {}", TESTED.get().is_none())
}

#[cfg_attr(rust_js, rust_js::nullable(name))]
fn set_cloned(name: Option<String>) {
    CLONED.set(name);
}

fn cloned() -> usize {
    let previous = CLONED.with_borrow(Clone::clone);
    let length = previous.as_ref().map_or(0, |n| n.len());
    CLONED.set(previous);
    length + CLONED.with_borrow(Clone::clone).map(|n| n.len()).unwrap_or(9)
}

#[cfg_attr(rust_js, rust_js::nullable(name))]
fn set_combined(name: Option<&'static str>) {
    COMBINED.set(name);
}

fn fallback() -> &'static str {
    "else"
}

fn combined() -> String {
    let mut shown = Vec::new();
    if COMBINED.get().is_some() {
        shown.push(COMBINED.get().unwrap().to_owned());
        shown.push(COMBINED.get().expect("set").to_owned());
    }
    shown.push(COMBINED.get().unwrap_or_default().to_owned());
    shown.push(COMBINED.get().unwrap_or_else(fallback).to_owned());
    shown.push(COMBINED.get().map_or_else(|| fallback().len(), str::len).to_string());
    shown.push(COMBINED.get().is_none_or(|n| n.is_empty()).to_string());
    shown.push(COMBINED.get().ok_or("err").is_ok().to_string());
    shown.push(COMBINED.get().ok_or_else(fallback).is_err().to_string());
    shown.push(COMBINED.get().filter(|n| n.len() > 1).is_some().to_string());
    shown.push(
        COMBINED
            .get()
            .and_then(|n| n.strip_prefix('b'))
            .unwrap_or("^")
            .to_owned(),
    );
    shown.push(COMBINED.get().or(Some("or")).unwrap_or("^").to_owned());
    shown.push(
        COMBINED
            .get()
            .or_else(|| fallback().strip_prefix('z'))
            .is_none()
            .to_string(),
    );
    shown.push(COMBINED.get().xor(Some("x")).unwrap_or("^").to_owned());
    shown.push(COMBINED.get().inspect(|n| assert!(!n.is_empty())).is_some().to_string());
    shown.push(COMBINED.get().map(str::len).unwrap_or(0).to_string());
    if let Some(name) = COMBINED.get()
        && name.len() > 1
    {
        shown.push(format!("long {name}"));
    }
    match COMBINED.get() {
        Some(name) if name.len() == 1 => shown.push("one".to_owned()),
        Some(_) => shown.push("other".to_owned()),
        None => shown.push("none".to_owned()),
    }
    shown.join(",")
}

#[cfg_attr(rust_js, rust_js::nullable(name))]
fn set_held(name: Option<&'static str>) {
    HELD.set(name);
}

fn held() -> bool {
    let holder = Holder { name: HELD.get() };
    holder.name.is_some()
}

#[cfg_attr(rust_js, rust_js::nullable(name))]
fn set_returned(name: Option<&'static str>) {
    RETURNED.set(name);
}

#[cfg_attr(rust_js, rust_js::nullable)]
fn returned() -> Option<&'static str> {
    RETURNED.get()
}

fn returned_held() -> bool {
    let holder = Holder { name: returned() };
    holder.name.is_some()
}

#[cfg_attr(rust_js, rust_js::nullable(name))]
fn set_compared(name: Option<&'static str>) {
    COMPARED.set(name);
}

fn compared() -> bool {
    COMPARED.get() == Some("a")
}

fn given(name: Option<&'static str>) {
    set_tested(name);
    set_cloned(name.map(str::to_owned));
    set_combined(name);
    set_held(name);
    set_returned(name);
    set_compared(name);
}

fn main() {
    let words = ["a", "bc"];
    for wanted in ["z", "a", "bc"] {
        given(words.iter().copied().find(|w| *w == wanted));
        println!(
            "{} {} {} {} {}",
            tested(),
            cloned(),
            held(),
            returned_held(),
            compared()
        );
        println!("{}", combined());
    }
    set_tested(None);
    set_held(None);
    println!("{} {}", tested(), held());
}
