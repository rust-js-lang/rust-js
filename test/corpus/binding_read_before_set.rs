// A match's binding of a variable the arm sets again is the variable where
// each read of it is before the variable is set, React's `newLast.next =
// clone; newLast = clone;`, and its own value where one is after.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Node {
    value: i32,
    next: RefCell<Option<Rc<Node>>>,
}

fn node(value: i32) -> Rc<Node> {
    Rc::new(Node {
        value,
        next: RefCell::new(None),
    })
}

// The binding read, then the variable set: one variable.
fn linked(values: &[i32]) -> Option<Rc<Node>> {
    let mut first: Option<Rc<Node>> = None;
    let mut new_last: Option<Rc<Node>> = None;
    for &value in values {
        let clone = node(value);
        match &new_last {
            None => {
                first = Some(Rc::clone(&clone));
                new_last = Some(clone);
            }
            Some(last) => {
                last.next.replace(Some(Rc::clone(&clone)));
                new_last = Some(clone);
            }
        }
    }
    first
}

// The variable set in one branch, the binding read after it in the other.
fn chosen(values: &[i32], flip: bool) -> i32 {
    let mut current: Option<Rc<Node>> = Some(node(100));
    let mut sum = 0;
    for &value in values {
        let next = node(value);
        match &current {
            None => current = Some(next),
            Some(held) if flip => {
                let old = Rc::clone(held);
                current = Some(next);
                sum += old.value;
            }
            Some(held) => {
                sum += held.value;
                current = Some(next);
            }
        }
    }
    sum
}

// A copy read after what it copied is set: its own value.
fn copied(steps: &[i32]) -> i32 {
    let total = Cell::new(0);
    let mut at = 1;
    for &step in steps {
        let before = at;
        at += step;
        total.set(total.get() + before * 10 + at);
    }
    total.get()
}

fn pair(a: i32, b: i32) -> String {
    format!("({a}, {b})")
}

// An argument read, then its variable set by a later one: the value it had.
fn argued(mut x: i32, flip: bool) -> String {
    let set = pair(x, {
        x += 1;
        x
    });
    let branched = pair(
        x,
        if flip {
            x += 10;
            x
        } else {
            x - 1
        },
    );
    format!("{set} {branched}")
}

// A binding read in a test after its variable is set: its own value.
fn retested(mut raw: Option<i32>) -> i32 {
    if let Some(error) = raw {
        raw = None;
        if error > 2 {
            return 7 + raw.unwrap_or(1);
        }
        return 3;
    }
    0
}

fn main() {
    println!("{} {}", argued(1, true), argued(1, false));
    println!("{} {} {}", retested(Some(5)), retested(Some(1)), retested(None));
    let mut walk = linked(&[1, 2, 3, 4]);
    let mut seen = Vec::new();
    while let Some(n) = walk {
        seen.push(n.value);
        walk = n.next.borrow().clone();
    }
    println!("{:?}", seen);
    println!("{} {}", chosen(&[1, 2, 3], false), chosen(&[1, 2, 3], true));
    println!("{}", copied(&[2, 3, 4]));
}
