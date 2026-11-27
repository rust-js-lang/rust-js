// `ToOwned::clone_into` writes its target, a slice's into a `Vec`, a
// `str`'s into a `String` and a `T: Clone`'s into a `T`: each was taken for
// a `to_owned()`, its target left as it was (#2627).
use std::cell::RefCell;

fn main() {
    let items = vec![1, 2, 3];
    let mut longer = vec![7, 8, 9, 10];
    items[..].clone_into(&mut longer);
    let mut shorter = vec![9];
    items[..].clone_into(&mut shorter);
    println!("{longer:?} {shorter:?}");
    let mut text = String::from("x");
    "abc".clone_into(&mut text);
    println!("{text}");
    let mut number = 0u8;
    5u8.clone_into(&mut number);
    println!("{number}");
    let cell = RefCell::new(7);
    let mut other = RefCell::new(0);
    cell.clone_into(&mut other);
    println!("{}", other.borrow());
    let mut names = vec![String::from("a")];
    [String::from("b"), String::from("c")][..].clone_into(&mut names);
    println!("{names:?}");
}
