// A reference to an object written whole through a `&mut` to it, `*m
// .as_mut().unwrap() = &other` of an `Option<&HashMap<..>>`, is another
// reference, never a copy of `other` into the map it was to, as
// `$assign(map, other)` made it; and what's written through a `RefCell`'s
// guard lent to a call that panics, `set_then_panic(&mut
// *slot.borrow_mut())`, is the cell's when a `catch_unwind` reads it, the
// guard's box copied back as it unwinds too, as a reference's is (#1021).
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

fn set_then_panic(slot: &mut Option<u32>) -> u32 {
    *slot = Some(1);
    panic!("no value")
}

fn text_in(root: &HashMap<String, Entry>, path: &str) -> Option<String> {
    let mut dir = Some(root);
    for part in path.split('/') {
        match dir.as_ref().unwrap().get(part)? {
            Entry::Dir(inner) => *dir.as_mut().unwrap() = inner,
            Entry::File(text) => return Some(text.clone()),
        }
    }
    None
}

enum Entry {
    Dir(HashMap<String, Entry>),
    File(String),
}

fn main() {
    let mut maps = HashMap::new();
    maps.insert(1, 5);
    let other = HashMap::from([(2, 6)]);
    let mut m = Some(&maps);
    *m.as_mut().unwrap() = &other;
    println!("{:?} {:?} {}", maps.get(&1), m.unwrap().get(&2), maps.len());

    let mut inner = HashMap::new();
    inner.insert("b.txt".to_string(), Entry::File("hello".to_string()));
    let mut root = HashMap::new();
    root.insert("a".to_string(), Entry::Dir(inner));
    root.insert("c.txt".to_string(), Entry::File("top".to_string()));
    for path in ["a/b.txt", "c.txt", "a/x"] {
        println!("{path}: {:?}", text_in(&root, path));
    }
    println!("{}", root.len());

    let slot = Rc::new(RefCell::new(None));
    let caught = panic::catch_unwind(AssertUnwindSafe(|| set_then_panic(&mut slot.borrow_mut())));
    println!("{} {:?}", caught.is_err(), slot.borrow());
}
