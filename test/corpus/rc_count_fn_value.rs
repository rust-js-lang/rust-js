use std::rc::{Rc, Weak};
fn main() {
    let names: Vec<Rc<String>> = vec![Rc::new("a".to_string())];
    let more = names.clone();
    println!("{:?}", names.iter().map(Rc::strong_count).collect::<Vec<_>>());
    let strs: Vec<Rc<str>> = vec![Rc::from("x")];
    let w: Vec<Weak<str>> = strs.iter().map(Rc::downgrade).collect();
    println!("{:?} {:?}", strs.iter().map(Rc::weak_count).collect::<Vec<_>>(), w.iter().map(Weak::strong_count).collect::<Vec<_>>());
    drop(more);
    println!("{:?}", names.iter().map(Rc::strong_count).sum::<usize>());
}
