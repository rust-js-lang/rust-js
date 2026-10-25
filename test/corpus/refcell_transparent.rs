// A `#[repr(transparent)]` struct of a `RefCell` of an object is its field
// (ADR 0584), and that cell is the object (ADR 0863): React's taint
// `RequestCleanupQueue`, an array pushed to through a set of them, is the
// array, `queue.push(v)`, never `queue.value.push(v)`. Of a named field and
// a tuple struct's, through an `Rc`, a `Vec`, a set of `&'static`s, a whole
// write, a `take` and a `replace`.
use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[repr(transparent)]
struct Queue {
    values: RefCell<Vec<String>>,
}

impl PartialEq for Queue {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Queue {}

impl Hash for Queue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

#[repr(transparent)]
struct Counts(RefCell<Vec<i32>>);

fn push_all(pending: &RefCell<HashSet<&'static Queue>>, value: &str) {
    pending.borrow().iter().for_each(|queue| {
        queue.values.borrow_mut().push(value.to_string());
    });
}

fn main() {
    let queue = Queue {
        values: RefCell::new(Vec::new()),
    };
    queue.values.borrow_mut().push("a".to_string());
    queue.values.borrow_mut().push("b".to_string());
    println!("{:?} {}", queue.values.borrow(), queue.values.borrow().len());

    let shared = Rc::new(Queue {
        values: RefCell::new(vec!["x".to_string()]),
    });
    let other = Rc::clone(&shared);
    other.values.borrow_mut().push("y".to_string());
    println!("{:?} {}", shared.values.borrow(), Rc::ptr_eq(&shared, &other));

    let counts = Counts(RefCell::new(vec![1]));
    counts.0.borrow_mut().push(2);
    *counts.0.borrow_mut() = vec![7, 8, 9];
    let taken = counts.0.take();
    let old = counts.0.replace(vec![4]);
    println!("{taken:?} {old:?} {:?}", counts.0.borrow());

    let first: &'static Queue = Box::leak(Box::new(Queue {
        values: RefCell::new(Vec::new()),
    }));
    let second: &'static Queue = Box::leak(Box::new(Queue {
        values: RefCell::new(vec!["held".to_string()]),
    }));
    let pending = RefCell::new(HashSet::new());
    pending.borrow_mut().insert(first);
    pending.borrow_mut().insert(second);
    pending.borrow_mut().insert(first);
    push_all(&pending, "v");
    println!("{} {:?} {:?}", pending.borrow().len(), first.values.borrow(), second.values.borrow());

    let queues = [
        Queue {
            values: RefCell::new(vec![]),
        },
        Queue {
            values: RefCell::new(vec!["q".to_string()]),
        },
    ];
    queues.iter().for_each(|q| q.values.borrow_mut().push("z".to_string()));
    let total: usize = queues.iter().map(|q| q.values.borrow().len()).sum();
    println!("{total}");
    local();
    newtypes();
}

fn push_one(queue: &Queue, value: &str) -> usize {
    queue.values.borrow_mut().push(value.to_string());
    queue.values.borrow().len()
}

fn local() {
    let queue = Queue {
        values: RefCell::new(Vec::new()),
    };
    println!("{}", push_one(&queue, "l"));
    queue.values.borrow_mut().push("m".to_string());
    println!("{:?}", queue.values.borrow());
}

// A `RefCell` of a transparent struct is its field's: the array of a
// `Vec`'s newtype, a box of a number's.
#[repr(transparent)]
#[derive(Debug, Default)]
struct Lines(Vec<String>);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default)]
struct Meters(f64);

fn newtypes() {
    let lines = Rc::new(RefCell::new(Lines(vec!["one".to_string()])));
    let alias = Rc::clone(&lines);
    alias.borrow_mut().0.push("two".to_string());
    let old = lines.replace(Lines(vec!["three".to_string()]));
    println!("{old:?} {:?} {}", lines.borrow(), Rc::ptr_eq(&lines, &alias));
    *alias.borrow_mut() = Lines(vec![]);
    let taken = lines.take();
    println!("{taken:?} {:?}", alias.borrow());
    let length = RefCell::new(Meters(1.5));
    let before = *length.borrow();
    length.borrow_mut().0 += 2.0;
    let old = length.replace(Meters(9.0));
    println!("{before:?} {old:?} {:?}", length.borrow());
}
