// An `Option`'s test says the `None` it can be (ADR 0298, amended by the
// user, 2026-10-28): a variable each of whose writes is a `Some(..)` or the
// crate's own `None`, `undefined`, is tested `x === undefined`; one that
// may hold what JS gave, a parameter's, a field's, what a `&mut` wrote or a
// call of the crate's gave, `x == null`, as either may be there. Each takes
// the branch Rust's does.

struct Slot {
    held: Option<u32>,
}

fn found(items: &[u32], wanted: u32) -> Option<usize> {
    items.iter().position(|&item| item == wanted)
}

fn set(slot: &mut Option<u32>, value: u32) {
    *slot = Some(value);
}

/// Made here, `None` or `Some(i)`: its `None` is `undefined`.
fn first_over(items: &[u32], limit: u32) -> u32 {
    let mut first = None;
    for &item in items {
        if first.is_none() && item > limit {
            first = Some(item);
        }
    }
    let Some(first) = first else {
        return 0;
    };
    first
}

/// A copy of one made here, and a borrow of it: the same.
fn copied(flag: bool) -> &'static str {
    let made = if flag { Some(1) } else { None };
    let copy = made;
    let borrowed = &copy;
    match borrowed {
        Some(_) => "some",
        None => "none",
    }
}

/// A parameter, which JS may give `null` or `undefined`: either.
fn given(value: Option<u32>) -> bool {
    let copy = value;
    copy.is_some()
}

/// A field's, a call's of the crate's and what a `&mut` wrote: either.
fn elsewhere(slot: &Slot, items: &[u32]) -> (bool, bool, bool) {
    let held = slot.held;
    let at = found(items, 3);
    let mut written = None;
    set(&mut written, 4);
    (held.is_none(), at.is_some(), written.is_some())
}

/// One made here or given: either.
fn mixed(value: Option<u32>, flag: bool) -> u32 {
    let chosen = if flag { value } else { None };
    match chosen {
        Some(n) => n,
        None => 7,
    }
}

fn main() {
    println!("{}", first_over(&[1, 5, 9], 4));
    println!("{}", first_over(&[1, 2], 4));
    println!("{} {}", copied(true), copied(false));
    println!("{} {}", given(Some(0)), given(None));
    let slot = Slot { held: None };
    println!("{:?}", elsewhere(&slot, &[1, 3]));
    println!("{:?}", elsewhere(&Slot { held: Some(0) }, &[]));
    println!("{} {} {}", mixed(Some(0), true), mixed(Some(2), false), mixed(None, true));
}
