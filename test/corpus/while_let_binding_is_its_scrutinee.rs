// A `while let Some(current) = child` whose binding is read only while
// `child` still holds it is `child`, by the scrutinee's name, `while (child
// != null) { if (child === skip) { child = child.sibling; continue; } .. }`,
// as React's TreeReflection walks a fiber's children (ADR 0434, amended): a
// branch that sets `child` and leaves the turn, by `continue`, `break` or
// `return`, is no way to a read after it. Where `current` is read once
// `child` holds another value, on any way, it stays a `const` of its own.
use std::rc::Rc;

struct Node {
    tag: u32,
    sibling: Option<Rc<Node>>,
}

fn count(mut child: Option<Rc<Node>>, skip: &Rc<Node>) -> u32 {
    let mut n = 0;
    while let Some(current) = child {
        if Rc::ptr_eq(&current, skip) {
            child = current.sibling.clone();
            continue;
        }
        if current.tag == 5 {
            n += 1;
        }
        child = current.sibling.clone();
    }
    n
}

// Left by `break` and by `return` too.
fn first_after(mut child: Option<Rc<Node>>, tag: u32) -> Option<u32> {
    while let Some(current) = child {
        if current.tag == tag {
            child = current.sibling.clone();
            break;
        }
        if current.tag == 0 {
            return Some(current.tag + 100);
        }
        child = current.sibling.clone();
    }
    child.map(|c| c.tag)
}

// `current` read after `child` is set on the way that goes on: its own.
fn read_after_set(mut child: Option<Rc<Node>>, log: &mut Vec<u32>) {
    while let Some(current) = child {
        if current.tag == 5 {
            child = current.sibling.clone();
            log.push(0);
        } else {
            child = None;
        }
        log.push(current.tag);
    }
}

// Set and left on one way, set and read after on the other, an `else` the
// program wrote: its own.
fn set_in_both(mut child: Option<Rc<Node>>, log: &mut Vec<u32>) {
    while let Some(current) = child {
        if current.tag == 5 {
            child = current.sibling.clone();
            continue;
        } else {
            child = match current.tag {
                7 => None,
                _ => current.sibling.clone(),
            };
        }
        log.push(current.tag);
    }
}

// Set on the way that leaves, read after on the other, the `else` the
// program wrote: the scrutinee.
fn left_in_then(mut child: Option<Rc<Node>>, log: &mut Vec<u32>) {
    while let Some(current) = child {
        if current.tag == 5 {
            child = current.sibling.clone();
            log.push(0);
            continue;
        } else {
            log.push(1);
        }
        log.push(current.tag);
        child = current.sibling.clone();
    }
}

// Of a reference, which a `while let` copies: set on one way that goes on,
// either branch, read after: its own.
fn set_in_then<'a>(mut child: Option<&'a Node>, log: &mut Vec<u32>) {
    while let Some(current) = child {
        if current.tag == 5 {
            child = current.sibling.as_deref();
            log.push(0);
        }
        log.push(current.tag);
        if current.tag != 5 {
            child = current.sibling.as_deref();
        }
    }
}

fn set_in_else<'a>(mut child: Option<&'a Node>, log: &mut Vec<u32>) {
    while let Some(current) = child {
        if current.tag == 5 {
            log.push(0);
        } else {
            child = current.sibling.as_deref();
            log.push(1);
        }
        log.push(current.tag);
        if current.tag == 5 {
            child = current.sibling.as_deref();
        }
    }
}

fn list(tags: &[u32]) -> (Option<Rc<Node>>, Vec<Rc<Node>>) {
    let mut last: Option<Rc<Node>> = None;
    let mut all = Vec::new();
    for &tag in tags.iter().rev() {
        let node = Rc::new(Node { tag, sibling: last });
        all.push(Rc::clone(&node));
        last = Some(node);
    }
    all.reverse();
    (last, all)
}

fn main() {
    let (head, all) = list(&[5, 1, 5, 5, 2]);
    println!("{}", count(head.clone(), &all[2]));
    println!("{}", count(head, &all[4]));
    println!("{:?}", first_after(list(&[1, 2, 3]).0, 2));
    println!("{:?}", first_after(list(&[1, 0, 3]).0, 2));
    println!("{:?}", first_after(list(&[1, 4]).0, 2));
    let mut log = Vec::new();
    read_after_set(list(&[5, 5, 3, 5]).0, &mut log);
    println!("{log:?}");
    let mut log = Vec::new();
    set_in_both(list(&[1, 5, 7, 2, 9, 7, 4, 5]).0, &mut log);
    println!("{log:?}");
    let mut log = Vec::new();
    left_in_then(list(&[1, 5, 7, 5, 4]).0, &mut log);
    println!("{log:?}");
    let (head, _all) = list(&[1, 5, 7, 5, 4]);
    let mut log = Vec::new();
    set_in_then(head.as_deref(), &mut log);
    println!("{log:?}");
    let mut log = Vec::new();
    set_in_else(head.as_deref(), &mut log);
    println!("{log:?}");
}
