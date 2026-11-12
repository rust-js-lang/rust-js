//! A library's types shown by their derived `Debug`s, a struct's and an
//! enum's, and a generic function that shows what it's given.

#[derive(Debug)]
pub struct Label(pub &'static str);

#[derive(Debug)]
pub enum Mode {
    Off,
    On(u32),
}

/// Shown by a `println!` of whatever it's given.
pub fn shown<T: std::fmt::Debug>(value: T) {
    println!("shown {value:?}");
}
