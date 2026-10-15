// A `NonNull<T>` is its raw pointer (ADR 0407), never null: a handle on its
// place. `new` is `None` of a null one, `as_ptr` and `new_unchecked` the
// pointer itself, `as_ref` and `as_mut` its place, as anyhow's and
// smallvec's take them.
use std::ptr::NonNull;

struct Node {
    value: i32,
    next: Option<NonNull<Node>>,
}

fn sum(mut at: Option<NonNull<Node>>) -> i32 {
    let mut total = 0;
    while let Some(node) = at {
        let node = unsafe { node.as_ref() };
        total += node.value;
        at = node.next;
    }
    total
}

fn main() {
    let last = Box::new(Node { value: 3, next: None });
    let last = NonNull::new(Box::into_raw(last)).unwrap();
    let first = Box::into_raw(Box::new(Node { value: 2, next: Some(last) }));
    let mut first = unsafe { NonNull::new_unchecked(first) };
    println!("{}", sum(Some(first)));
    unsafe { first.as_mut().value = 10 };
    println!("{}", sum(Some(first)));
    let raw: *mut Node = first.as_ptr();
    println!("{}", unsafe { (*raw).value });
    println!("{} {}", NonNull::new(std::ptr::null_mut::<Node>()).is_none(), first == unsafe { NonNull::new_unchecked(raw) });
    let mut n = 5;
    let mut handle = NonNull::from(&mut n);
    unsafe { *handle.as_mut() += 1 };
    println!("{n}");
    // Of an object: a handle on it, whose `as_mut()` is the object.
    let mut node = Node { value: 1, next: None };
    let mut at = NonNull::from(&mut node);
    unsafe { at.as_mut().value = 7 };
    println!("{}", node.value);
    unsafe {
        drop(Box::from_raw(first.as_ptr()));
        drop(Box::from_raw(last.as_ptr()));
    }
}
