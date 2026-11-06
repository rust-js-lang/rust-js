// A `#[rust_js::nullable]` `Option<Option<T>>` field of a `T` that may look
// like `None`, a `Weak` (`Weak::new()` is `null`), a `Pin<Box<()>>`, a
// `MaybeUninit`, a `()` and a type parameter's: its `None` is `undefined`
// and its `Some(None)` `null`, as any such field's, and its `Some(Some(v))`
// ADR 0051's box of the inner `Some`, `$some(v)`, as a plain `Option` of
// it is, so the three states stay apart, where it's made, given to a
// function, given back, set and read (#836). Of a struct kept to the
// crate: a `pub` one's `.d.ts` would declare the field `T | null`, which
// the box isn't, so it's refused.
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::rc::{Rc, Weak};

struct Node {
    v: i32,
}

struct Holder {
    #[cfg_attr(rust_js, rust_js::nullable)]
    weak: Option<Option<Weak<Node>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pin: Option<Option<Pin<Box<()>>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    uninit: Option<Option<MaybeUninit<u8>>>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    unit: Option<Option<()>>,
}

fn state<T>(o: &Option<Option<T>>) -> &'static str {
    match o {
        None => "none",
        Some(None) => "some none",
        Some(Some(_)) => "some some",
    }
}

fn show(h: &Holder) {
    let w = match &h.weak {
        Some(Some(w)) => format!("{:?}", w.upgrade().map(|n| n.v)),
        other => state(other).to_string(),
    };
    println!("{w} {} {} {}", state(&h.pin), state(&h.uninit), state(&h.unit));
}

fn make(i: u32, n: &Rc<Node>) -> Holder {
    match i {
        0 => Holder { weak: Some(Some(Weak::new())), pin: Some(Some(Box::pin(()))), uninit: Some(Some(MaybeUninit::uninit())), unit: Some(Some(())) },
        1 => Holder { weak: Some(Some(Rc::downgrade(n))), pin: Some(None), uninit: Some(None), unit: Some(None) },
        2 => Holder { weak: Some(None), pin: None, uninit: None, unit: None },
        _ => Holder { weak: None, pin: Some(Some(Box::pin(()))), uninit: Some(Some(MaybeUninit::new(1))), unit: Some(Some(())) },
    }
}

fn given(weak: Option<Option<Weak<Node>>>, unit: Option<Option<()>>) -> Holder {
    Holder { weak, pin: None, uninit: None, unit }
}

fn taken(h: Holder) -> (Option<Option<Weak<Node>>>, Option<Option<()>>) {
    (h.weak, h.unit)
}

fn main() {
    let n = Rc::new(Node { v: 7 });
    for i in 0..4 {
        show(&make(i, &n));
    }
    for (w, u) in [(Some(Some(Weak::new())), Some(Some(()))), (Some(None), Some(None)), (None, None)] {
        let h = given(w, u);
        show(&h);
        let (w, u) = taken(h);
        println!("{} {}", state(&w), state(&u));
    }
    let mut h = make(2, &n);
    h.weak = Some(Some(Rc::downgrade(&n)));
    h.unit = Some(Some(()));
    show(&h);
    h.weak = Some(None);
    h.unit = None;
    show(&h);
    generics();
}

struct Generic<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    held: Option<Option<T>>,
}

fn generic_state<T>(g: &Generic<T>) -> &'static str {
    state(&g.held)
}

fn generic_made<T>(held: Option<Option<T>>) -> Generic<T> {
    Generic { held }
}

pub fn generics() {
    for g in [generic_made(Some(Some(()))), generic_made(Some(None)), generic_made(None)] {
        print!("{} ", generic_state(&g));
    }
    for g in [generic_made(Some(Some(3u32))), generic_made(Some(None)), generic_made(None)] {
        print!("{} ", generic_state(&g));
    }
    let g: Generic<Option<u8>> = generic_made(Some(Some(None)));
    println!("{} {:?}", generic_state(&g), g.held);
}
