//! A consumer comparing a library's types through `Option`s, references,
//! `Rc`s and the library's generic functions: each by the library's own
//! `PartialEq`, as React's `processDispatchQueueItemsInOrder` compares
//! its listeners' fibers.
use std::rc::Rc;

use dep::{Dispatch, Fiber, Point, Update, Version, make, update};

fn same_instance(previous: Option<&Option<Rc<Fiber>>>, instance: &Option<Rc<Fiber>>) -> bool {
    previous == Some(instance)
}

struct Listener {
    instance: Option<Rc<Fiber>>,
}

fn dispatched(listeners: &[Listener], stopped: bool) -> u32 {
    let mut count = 0;
    let mut previous_instance = None;
    for Listener { instance } in listeners.iter().rev() {
        if previous_instance != Some(instance) && stopped {
            return count;
        }
        count += 1;
        previous_instance = Some(instance);
    }
    count
}

pub fn main() {
    let a = Some(make(1));
    let b = Some(make(1));
    println!("{} {}", same_instance(Some(&a), &a), same_instance(Some(&a), &b));
    println!("{} {}", same_instance(None, &a), same_instance(Some(&None), &None));
    // A cycle, which a walk of the fibers' fields never leaves.
    let parent = make(1);
    let child = make(2);
    *child.parent.borrow_mut() = Some(Rc::clone(&parent));
    *parent.parent.borrow_mut() = Some(Rc::clone(&child));
    let other = make(2);
    *other.parent.borrow_mut() = Some(Rc::clone(&parent));
    println!("{}", same_instance(Some(&Some(Rc::clone(&child))), &Some(other)));
    let shared = make(3);
    let listeners = [
        Listener {
            instance: Some(Rc::clone(&shared)),
        },
        Listener {
            instance: Some(Rc::clone(&shared)),
        },
        Listener {
            instance: Some(make(3)),
        },
    ];
    println!("{} {}", dispatched(&listeners, false), dispatched(&listeners, true));
    // An `Option<&Rc<Fiber>>`.
    let fresh = make(3);
    let held: Option<&Rc<Fiber>> = Some(&shared);
    println!("{} {}", held == Some(&shared), held == Some(&fresh));
    // Nested `Option`s.
    let nested: Option<Option<Rc<Fiber>>> = Some(Some(Rc::clone(&shared)));
    let unlike: Option<Option<Rc<Fiber>>> = Some(Some(make(3)));
    println!(
        "{} {} {}",
        nested == Some(Some(Rc::clone(&shared))),
        nested == unlike,
        nested == Some(None)
    );
    // A `PartialEq` alone of its address, of a cycle too.
    let first = update(1);
    let second = update(1);
    *first.next.borrow_mut() = Some(Rc::clone(&second));
    *second.next.borrow_mut() = Some(Rc::clone(&first));
    let pending: Option<&Option<Rc<Update>>> = Some(&first.next.borrow().clone());
    println!(
        "{} {} {}",
        pending == Some(&Some(Rc::clone(&second))),
        pending == Some(&Some(Rc::clone(&first))),
        Some(Some(Rc::clone(&first))) == Some(Some(update(1)))
    );
    *first.next.borrow_mut() = None;
    // A derived `PartialEq`, field by field.
    let p = Point {
        x: 1,
        label: "a".to_string(),
    };
    let q = Point {
        x: 1,
        label: "a".to_string(),
    };
    let r = Point {
        x: 1,
        label: "b".to_string(),
    };
    println!("{} {} {}", p == q, p == r, Some(&p) == Some(&q));
    // A library's derived `PartialEq` of a part equal only to itself.
    let click = |instance: &Rc<Fiber>| Dispatch {
        instance: Some(Rc::clone(instance)),
        name: "click".to_string(),
    };
    let (one, two) = (make(5), make(5));
    println!(
        "{} {} {}",
        click(&one) == click(&one),
        click(&one) == click(&two),
        Some(click(&one)) == Some(click(&two))
    );
    // A hand-written `eq`, through an `Option` and a `Vec`.
    let v1 = Version { major: 1, minor: 0 };
    let v2 = Version { major: 1, minor: 2 };
    let v3 = Version { major: 2, minor: 0 };
    println!(
        "{} {} {} {}",
        v1 == v2,
        Some(&v1) == Some(&v3),
        v1.minor + v2.minor,
        vec![v1] == vec![v2]
    );
    // The library's generic code, given each.
    println!("{} {}", dep::same(&shared, &shared), dep::same(&shared, &fresh));
    println!(
        "{} {}",
        dep::same(&Some(Rc::clone(&shared)), &Some(Rc::clone(&fresh))),
        dep::same(&p, &q)
    );
    let fibers = [make(4), Rc::clone(&shared), make(3)];
    println!(
        "{:?} {:?} {:?}",
        dep::first_index(&fibers, &shared),
        dep::first_index(&fibers, &fresh),
        dep::first_index(&[p, r], &q)
    );
    let w = Version { major: 2, minor: 9 };
    println!(
        "{:?} {}",
        dep::first_index(&[Version { major: 1, minor: 0 }, v3], &w),
        w.minor
    );
}
