// Two `Option`s of a JS value of any shape are equal as std's `Option`s
// are, whichever of `null` and `undefined` JS has each `None` as: both
// `None`, or each a `Some` of the same value, `===` (ADR 0612, amended).
// A value of any shape in a struct, a tuple or a slice is compared so
// too, by `===`, not by what's in it.

use std::marker::PhantomData;

/// A JS value of any shape, as `js::Unknown` is: never `null` nor
/// `undefined`, which an `Option` of one is `None` of.
#[cfg_attr(rust_js, rust_js::types = "unknown")]
struct Value(PhantomData<*mut ()>, u8);

/// The same value, as JS's `===` has it.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `JSON.parse(text)`: `null` of `"null"`, a `None`, and a new object of
/// `"{}"`.
#[cfg_attr(rust_js, rust_js::link_name = "JSON.parse")]
fn parse(text: &str) -> Option<&'static Value> {
    match text {
        "null" => None,
        _ => Some(Box::leak(Box::new(Value(PhantomData, 0)))),
    }
}

/// `this.missing`, which no object has: `undefined`, a `None`.
#[cfg_attr(rust_js, rust_js::link_name = "get missing")]
fn missing(this: &Value) -> Option<&'static Value> {
    let _ = this;
    None
}

#[derive(PartialEq)]
struct Pair<'a> {
    value: &'a Value,
    maybe: Option<&'a Value>,
}

fn equal(a: Option<&Value>, b: Option<&Value>) -> bool {
    a == b
}

fn differ(a: Option<&Value>, b: Option<&Value>) -> bool {
    a != b
}

fn found(items: &[Option<&Value>], item: Option<&Value>) -> bool {
    items.contains(&item)
}

fn same_items(a: &[Option<&Value>; 2], b: &[Option<&Value>; 2]) -> bool {
    a == b
}

fn same_pairs(a: (Option<&Value>, i32), b: (Option<&Value>, i32)) -> bool {
    a == b
}

fn same_somes(a: Option<Option<&Value>>, b: Option<Option<&Value>>) -> bool {
    a == b
}

fn same(a: Pair, b: Pair) -> bool {
    a == b
}

fn main() {
    let object = parse("{}").unwrap();
    let other = parse("{}").unwrap();
    let nil = parse("null");
    let absent = missing(object);
    // `null` and `undefined` are each `None`.
    println!("{} {}", equal(nil, absent), differ(nil, absent));
    println!("{} {}", nil == missing(other), missing(other) != nil);
    println!("{} {}", nil == None, absent == None);
    // A value is itself, and no other value, nor `None`.
    println!(
        "{} {}",
        equal(Some(object), Some(object)),
        equal(Some(object), Some(other))
    );
    println!("{} {}", equal(Some(object), nil), differ(absent, Some(other)));
    println!("{} {}", parse("{}") == parse("{}"), nil == Some(object));
    // In a slice, a tuple, an `Option` and a struct, the same.
    println!(
        "{} {}",
        found(&[nil, Some(object)], absent),
        found(&[Some(other)], Some(object))
    );
    println!(
        "{} {}",
        same_items(&[nil, Some(object)], &[absent, Some(object)]),
        same_pairs((nil, 1), (absent, 1))
    );
    println!(
        "{} {}",
        same_somes(Some(nil), Some(absent)),
        same_somes(Some(Some(object)), Some(Some(other)))
    );
    let pair = |value, maybe| Pair { value, maybe };
    println!(
        "{} {}",
        same(pair(object, nil), pair(object, absent)),
        same(pair(object, nil), pair(other, nil))
    );
    println!("{}", same(pair(object, Some(object)), pair(object, Some(other))));
}
