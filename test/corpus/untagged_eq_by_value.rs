// An untagged enum whose `==` is derived, of payloads JS's `===` compares
// as Rust's `==` does, is compared by JS's own, as React's `child.key ===
// key` of a `ReactKey`, a string or the symbol a renderer gives it: its
// value is its payload, each variant's told from the others' by what it
// is (ADR 0214, amended). An `Option` of one by `==`, its `None` `null` or
// `undefined`. A float's `NaN` is equal to nothing, as Rust's; an enum of
// a payload compared otherwise, a `Vec`'s, or of a hand-written `==`, is
// compared as before.
use std::cell::RefCell;

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Debug, PartialEq)]
enum Key {
    Text(String),
    Index(u32),
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Size {
    Count(f64),
    Named(&'static str),
    #[cfg_attr(rust_js, rust_js::null)]
    Unset,
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Debug, PartialEq)]
enum Items {
    Some(Vec<u32>),
    Count(u32),
}

struct Fiber {
    key: RefCell<Option<Key>>,
}

fn same_key(a: &Fiber, b: &Fiber) -> bool {
    *a.key.borrow() == *b.key.borrow()
}

fn main() {
    let text = Key::Text("a".to_string());
    let optimistic = Key::Index(7);
    println!(
        "{} {} {} {}",
        text == Key::Text("a".to_string()),
        text == Key::Text("b".to_string()),
        optimistic == Key::Index(7),
        optimistic == text
    );
    let fibers = [
        Fiber { key: RefCell::new(None) },
        Fiber { key: RefCell::new(Some(text.clone())) },
        Fiber { key: RefCell::new(Some(Key::Text("a".to_string()))) },
        Fiber { key: RefCell::new(Some(optimistic)) },
        Fiber { key: RefCell::new(None) },
        Fiber { key: RefCell::new(Some(Key::Text("7".to_string()))) },
    ];
    for a in &fibers {
        let row: Vec<bool> = fibers.iter().map(|b| same_key(a, b)).collect();
        println!("{row:?}");
    }
    let sizes = [Size::Count(1.0), Size::Count(f64::NAN), Size::Named("1"), Size::Unset];
    for a in sizes {
        let row: Vec<bool> = sizes.iter().map(|&b| a == b).collect();
        println!("{a:?} {row:?}");
    }
    // `NaN` isn't found, as Rust's `==` finds none.
    println!("{} {}", sizes.contains(&Size::Count(f64::NAN)), sizes.contains(&Size::Named("1")));
    let items = Items::Some(vec![1, 2]);
    println!("{} {}", items == Items::Some(vec![1, 2]), items == Items::Count(2));
}
