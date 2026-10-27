// An untagged enum's variant without fields may be JS's `undefined`,
// `#[rust_js::undefined]`, or `null`, `#[rust_js::null]` (ADR 0676), as
// `js::Kind`'s `Undefined` and `Null` are: each its own value, told apart
// by `===`, and an `Option` of one a box, as its `None` is neither.

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum Field<'a> {
    Text(&'a str),
    Count(f64),
    #[cfg_attr(rust_js, rust_js::undefined)]
    Missing,
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared,
}

fn describe(field: Field) -> String {
    match field {
        Field::Text(text) => format!("text {text}"),
        Field::Count(n) => format!("count {n}"),
        Field::Missing => "missing".to_string(),
        Field::Cleared => "cleared".to_string(),
    }
}

fn nothing(field: Field) -> bool {
    matches!(field, Field::Missing | Field::Cleared)
}

// One whose payload JS can't compare, a `Vec`, compares by its derived
// `==`; a unit variant, a constant, by `===` still: `undefined` isn't
// `null` (ADR 0676).
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Debug, PartialEq)]
enum Slot {
    Items(Vec<i32>),
    #[cfg_attr(rust_js, rust_js::undefined)]
    Missing,
    #[cfg_attr(rust_js, rust_js::null)]
    Cleared,
}

fn is_missing(slot: &Slot) -> bool {
    *slot == Slot::Missing
}

fn main() {
    let fields = [Field::Text("a"), Field::Count(2.0), Field::Missing, Field::Cleared];
    for field in fields {
        println!("{} {field:?} {}", describe(field), nothing(field));
    }
    // `undefined` isn't `null`, and each is itself.
    println!(
        "{} {} {}",
        Field::Missing == Field::Cleared,
        Field::Cleared == Field::Cleared,
        Field::Missing == Field::Missing
    );
    // An `Option` of one: `Some(Missing)` isn't `None`.
    let found = [None, Some(Field::Missing), Some(Field::Cleared), Some(Field::Count(1.0))];
    for field in found {
        println!("{field:?} {} {}", field.is_some(), field.map(describe).unwrap_or_default());
    }
    let slots = [Slot::Items(vec![1]), Slot::Missing, Slot::Cleared];
    for slot in &slots {
        println!("{slot:?} {} {}", is_missing(slot), *slot == Slot::Cleared);
    }
}
