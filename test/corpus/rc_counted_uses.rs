//@ library-refused: another crate may share
// Counted `Rc`s and `Arc`s where other code meets them (ADR 0320): a value
// with a destructor dropped by the last one, generic code, which counts
// every `Rc`, traits, loops, `new_cyclic`, and `make_mut` of a field.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Weak};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
struct Doc {
    title: Rc<String>,
    pages: Rc<Vec<u32>>,
}

struct Owner {
    me: Weak<Owner>,
    name: String,
}

fn count<T>(rc: &Rc<T>) -> usize {
    Rc::strong_count(rc)
}

fn main() {
    let noisy = Rc::new(Noisy("a"));
    let again = Rc::clone(&noisy);
    drop(noisy);
    println!("one left: {}", Rc::strong_count(&again));
    drop(again);
    println!("none left");

    let doc = Doc { title: Rc::new("rust".to_string()), pages: Rc::new(vec![1, 2]) };
    let mut copy = doc.clone();
    println!("{} {} {:?}", count(&doc.title), copy == doc, copy.partial_cmp(&doc));
    let fresh = Doc { title: Rc::new("rust".to_string()), pages: Rc::new(vec![1, 2]) };
    println!("{} {}", fresh == doc, Rc::new(1) == Rc::new(1));
    Rc::make_mut(&mut copy.pages).push(3);
    println!("{:?} {:?} {}", doc, copy, count(&doc.pages));

    let lists: Vec<Rc<Vec<u8>>> = vec![vec![1], vec![2, 3]].into_iter().map(Rc::new).collect();
    let mut total = 0;
    for list in lists.iter() {
        for item in list.iter() {
            total += item;
        }
    }
    println!("{total} {}", lists.iter().map(|l| count(l)).sum::<usize>());

    let shared = Rc::new(RefCell::new(vec![1]));
    let other = Rc::clone(&shared);
    other.borrow_mut().push(2);
    println!("{:?} {}", shared.borrow(), count(&shared));
    let owned: Vec<i32> = Rc::unwrap_or_clone(shared).into_inner();
    println!("{owned:?} {:?} {}", other.borrow(), count(&other));

    let owner = Arc::new_cyclic(|me: &Weak<Owner>| Owner { me: me.clone(), name: "self".to_string() });
    let me = owner.me.upgrade().unwrap();
    println!("{} {} {}", me.name, Arc::strong_count(&owner), Arc::weak_count(&owner));
    drop(me);
    println!("{} {}", Arc::strong_count(&owner), Arc::ptr_eq(&owner, &owner.me.upgrade().unwrap()));
    let defaulted: Rc<u32> = Rc::default();
    println!("{defaulted} {}", Rc::new(5) > defaulted);
    let w = Rc::downgrade(&defaulted);
    println!("{}", Rc::weak_count(&defaulted));
    drop(w);
    println!("{}", Rc::weak_count(&defaulted));
    let two = Rc::new(2);
    let also = Rc::clone(&two);
    println!("{:?}", Rc::try_unwrap(two));
    println!("{}", Rc::strong_count(&also));
    let counter = Rc::new(Cell::new(0));
    println!("{}", Rc::strong_count(&counter));
    let cell = Rc::new(Cell::new(1));
    let alias = Rc::clone(&cell);
    alias.set(5);
    println!("{}", cell.get());
}
