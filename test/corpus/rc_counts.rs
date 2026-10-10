//@ library-refused: another crate may share
// An `Rc` whose counts the crate reads is `{ value, strong, weak }`, counted
// as Rust counts it: cloned, dropped, downgraded, upgraded and unwrapped.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}

fn main() {
    let a = Rc::new(5);
    println!("{} {}", Rc::strong_count(&a), Rc::weak_count(&a));
    let b = Rc::clone(&a);
    {
        let c = a.clone();
        println!("{} {}", Rc::strong_count(&a), *c + *b);
    }
    println!("{} {}", Rc::strong_count(&a), Rc::ptr_eq(&a, &b));
    drop(b);
    println!("{}", Rc::strong_count(&a));

    let mut a = a;
    *Rc::get_mut(&mut a).unwrap() += 1;
    let weak = Rc::downgrade(&a);
    println!("{} {:?}", Rc::weak_count(&a), weak.upgrade().map(|n| *n));
    let shared = a.clone();
    println!("{:?}", Rc::get_mut(&mut a));
    println!("{}", *Rc::make_mut(&mut a) + 1);
    *Rc::make_mut(&mut a) += 10;
    println!("{} {} {}", a, shared, Rc::ptr_eq(&a, &shared));
    println!("{:?}", Rc::try_unwrap(shared));
    println!("{:?} {:?}", weak.upgrade(), Rc::into_inner(a));
    println!("{:?} {} {}", weak.upgrade(), weak.strong_count(), weak.weak_count());

    let leaf = Rc::new(Node {
        name: "leaf".to_string(),
        parent: RefCell::new(Weak::new()),
        children: RefCell::new(vec![]),
    });
    println!("{:?}", leaf.parent.borrow().upgrade().map(|p| p.name.clone()));
    {
        let branch = Rc::new(Node {
            name: "branch".to_string(),
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(vec![Rc::clone(&leaf)]),
        });
        *leaf.parent.borrow_mut() = Rc::downgrade(&branch);
        println!(
            "{:?} {} {} {}",
            leaf.parent.borrow().upgrade().map(|p| p.name.clone()),
            Rc::strong_count(&branch),
            Rc::weak_count(&branch),
            branch.children.borrow().len()
        );
        println!("{}", Rc::strong_count(&leaf));
    }
    println!("{:?} {}", leaf.parent.borrow().upgrade().map(|p| p.name.clone()), Rc::strong_count(&leaf));

    let text = Rc::new(String::from("hi"));
    let other = Rc::clone(&text);
    println!("{} {}", Rc::unwrap_or_clone(text), Rc::strong_count(&other));
}
