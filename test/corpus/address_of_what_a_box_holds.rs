//@ native: wasm32
// What a `Box`, an `Rc` or a `Vec` field holds is an allocation of its own,
// apart from the field, as it is in JS: a borrow of the field records no
// object as stored in it. The object a box held, in JS its value, was
// recorded where the field is, so `&raw const s.b` and `&raw const *s.b`
// were one address, `true` where native Rust gives `false` (#1114).
use std::rc::Rc;

#[repr(C)]
#[derive(Debug)]
struct P {
    a: u8,
    b: u32,
}

#[derive(Debug)]
struct S {
    n: u8,
    boxed: Box<P>,
    shared: Rc<P>,
    items: Vec<P>,
    maybe: Option<Box<P>>,
}

// A tuple struct, an array in JS, held by an `Rc` a tuple struct holds.
#[repr(C)]
struct Q(u8, u32);

struct Pair(u8, Rc<Q>, u64);

fn main() {
    let s = S {
        n: 1,
        boxed: Box::new(P { a: 2, b: 3 }),
        shared: Rc::new(P { a: 4, b: 5 }),
        items: vec![P { a: 6, b: 7 }],
        maybe: Some(Box::new(P { a: 8, b: 9 })),
    };
    let start = &raw const s as usize;
    let end = start + std::mem::size_of::<S>();
    let outside = |at: usize| at < start || at >= end;
    println!("box {}", outside(&raw const *s.boxed as usize));
    println!("rc {}", outside(&raw const *s.shared as usize));
    println!("vec {}", outside(s.items.as_ptr() as usize));
    println!("option box {}", outside(s.maybe.as_deref().unwrap() as *const P as usize));
    println!("fields inside {}", !outside(&raw const s.boxed as usize) && !outside(&raw const s.shared as usize));
    let pair = Pair(10, Rc::new(Q(11, 12)), 13);
    let at = &raw const pair as usize;
    let lent = &raw const *pair.1 as usize;
    println!("tuple rc {}", lent < at || lent >= at + std::mem::size_of::<Pair>());
    println!("{s:?} {} {} {}", pair.0, pair.1.0, pair.1.1 + pair.2 as u32);
}
