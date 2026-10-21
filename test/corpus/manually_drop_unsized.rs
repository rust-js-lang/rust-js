// A struct whose last field is a `dyn`, shown through a reference to it
// (ADR 0409): of a plain `T`, `Display` and `ToString`, and of a `ManuallyDrop<T>`, which is the
// value it holds (ADR 0571). The field reads as its value and the `impl`
// the struct was given, `labeled.impl.fmt(labeled.value)`.
use std::fmt::Display;
use std::mem::ManuallyDrop;

struct Tagged<T: ?Sized> {
    tag: u8,
    value: T,
}

struct Labeled<T: ?Sized> {
    tag: u8,
    value: ManuallyDrop<T>,
}

fn tagged(tagged: &Tagged<dyn Display>) -> String {
    format!("{} {}", tagged.tag, &tagged.value)
}

fn named(tagged: &Tagged<dyn ToString>) -> String {
    tagged.value.to_string()
}

fn labeled(labeled: &Labeled<dyn Display>) -> String {
    format!("{} {}", labeled.tag, &*labeled.value)
}

fn main() {
    println!("{}", tagged(&Tagged { tag: 1, value: 5 }));
    println!("{}", tagged(&Tagged { tag: 2, value: "five" }));
    println!("{}", named(&Tagged { tag: 0, value: 'x' }));
    let number = Labeled {
        tag: 3,
        value: ManuallyDrop::new(5),
    };
    let text = Labeled {
        tag: 4,
        value: ManuallyDrop::new("five"),
    };
    println!("{}", labeled(&number));
    println!("{}", labeled(&text));
}
