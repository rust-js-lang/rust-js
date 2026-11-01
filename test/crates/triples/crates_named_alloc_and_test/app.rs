//! A crate calling its dependencies named `alloc` and `test` (#455), whose
//! items are theirs, and std's alike, each what it does (ADR 1054).
pub fn main() {
    println!("{} {}", test::black_box(2), std::hint::black_box(2));
    println!("{} {}", alloc::fmt::format("a"), format!("{}", "a"));
    println!("{}", test::wrapped("b"));
    let mine = alloc::string::String(3);
    println!("{:?} {}", mine.clone(), mine == alloc::string::String(3));
    println!("{} {}", alloc::named(&mine), alloc::named(&String::from("s")));
}
