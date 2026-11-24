//! A consumer that changes the part in place, through a function of its
//! own given `&mut q.left`: so it compiles the library's functions itself,
//! each copy made, as the contract says the part's copy was left out.
//! Recorded of the struct alone, it would call the library's JS, and the
//! write through `q.left` would show in `p` (#2204, mutation
//! `elided-walk-parts-unrecorded`).
use dep::{Inner, Pair, copied, twice};

fn bump(inner: &mut Inner) {
    inner.n += 10;
}

pub fn main() {
    let p = Pair {
        left: Inner { n: 1 },
        tag: 7,
    };
    let mut q = copied(&p);
    bump(&mut q.left);
    let (mut a, b) = twice(p);
    bump(&mut a.left);
    println!("{} {} {} {} {}", p.left.n, q.left.n, a.left.n, b.left.n, p.tag);
}
