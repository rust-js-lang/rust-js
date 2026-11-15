// A crate's own items named as std's are, `Sized` and `Box`, as
// a UI kit's `Sized` of a flattened range is (#1820): a program that names
// them means its own, and code written beside it names std's by their
// paths, `::core::marker::Sized`, as the declarations' conformance harness
// does, whose `?Sized` a crate's `pub struct Sized` broke.

/// What a box's width is, scaled.
#[derive(Clone, Debug, PartialEq)]
pub struct Sized {
    pub width: u32,
}

/// A box of a label of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Box {
    pub label: String,
}

pub fn area(sized: &Sized, scale: Option<u32>) -> u32 {
    sized.width * scale.unwrap_or(1)
}

pub fn labelled(boxed: &Box, suffix: Option<&str>) -> String {
    format!("{}{}", boxed.label, suffix.unwrap_or(""))
}

fn main() {
    println!("{}", area(&Sized { width: 3 }, Some(2)));
    println!("{}", area(&Sized { width: 4 }, None));
    let boxed = Box { label: "a".to_string() };
    println!("{}", labelled(&boxed, Some("!")));
    println!("{}", labelled(&boxed, None));
}
