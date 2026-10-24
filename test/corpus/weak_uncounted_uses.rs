// An uncounted `Weak`, of an `Rc` nothing asks a count of, `upgrade` or
// `strong_count`, is the object itself, or `null` of a `Weak::new()` (ADR
// 0843): an `Option` of one boxes its `Some`, a clone of a struct holding
// one shares it, one written through a `&mut`, replaced or taken, and
// `Weak::ptr_eq` is identity, two `Weak::new()`s equal, as std's are.

use std::rc::{Rc, Weak};

trait Named {
    fn name(&self) -> String;
}

struct Node {
    label: String,
    parent: Weak<Node>,
}

impl Named for Node {
    fn name(&self) -> String {
        self.label.clone()
    }
}

#[derive(Clone)]
struct Link {
    to: Weak<Node>,
    tag: u8,
}

fn point(slot: &mut Weak<Node>, at: &Rc<Node>) {
    *slot = Rc::downgrade(at);
}

fn main() {
    let root = Rc::new(Node {
        label: "root".into(),
        parent: Weak::new(),
    });
    let kid = Rc::new(Node {
        label: "kid".into(),
        parent: Rc::downgrade(&root),
    });
    println!("{}", kid.name());
    let maybe: Option<Weak<Node>> = Some(Weak::new());
    let none: Option<Weak<Node>> = None;
    println!("{} {}", maybe.is_some(), none.is_some());
    let mut links = vec![Link {
        to: Weak::new(),
        tag: 1,
    }];
    let copy = links[0].clone();
    point(&mut links[0].to, &root);
    println!(
        "{} {}",
        Weak::ptr_eq(&links[0].to, &kid.parent),
        Weak::ptr_eq(&copy.to, &kid.parent)
    );
    println!("{} {}", Weak::ptr_eq(&copy.to, &Weak::new()), copy.tag);
    let mut w = Rc::downgrade(&kid);
    let old = std::mem::replace(&mut w, Weak::new());
    println!(
        "{} {}",
        Weak::ptr_eq(&old, &Rc::downgrade(&kid)),
        Weak::ptr_eq(&w, &Weak::new())
    );
    let taken = std::mem::take(&mut links[0].to);
    println!(
        "{} {}",
        Weak::ptr_eq(&taken, &kid.parent),
        Weak::ptr_eq(&links[0].to, &Weak::new())
    );
    let ws: Vec<Weak<Node>> = vec![Rc::downgrade(&root); 3];
    println!("{}", ws.iter().all(|w| Weak::ptr_eq(w, &kid.parent)));
}
