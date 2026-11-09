//! Each dropped as Rust drops it (#1051).
use dep::{Tag, node, tagged};

pub fn main() {
    let tag = Tag("given");
    let made = node(move || {
        let tag = &tag;
        tag.0.len() as u32
    });
    println!("{}", (made.run)());
    drop(made);
    println!("{}", tagged()());
}
