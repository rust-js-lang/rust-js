//@ native: wasm32
// A local whose address is taken, moved into another: the local moved to is
// its own allocation, at another address, as each live local is (#1535). So
// the move copies, and the local moved out keeps its object: assigned again,
// it's written over in place, at its own address still, and the value moved
// keeps what it was; where it may be unmade, it's made there (#1500).

#[derive(Debug)]
struct N(u8, u32);

#[derive(Debug)]
struct Outer {
    inner: N,
    tag: u16,
}

fn address_in(n: N) -> (usize, u32) {
    (&raw const n as usize, n.1)
}

/// Made on one way alone, `made`, then assigned: made where it's unmade,
/// written over where it's made.
fn maybe_made(made: bool) -> (bool, N) {
    let mut d;
    let mut at = 0;
    if made {
        d = N(22, 23);
        at = &raw const d as usize;
        println!("{d:?}");
    }
    d = N(24, 25);
    (!made || at == &raw const d as usize, d)
}

/// One with drop glue, which rustc drops before a write over it: made on
/// one way alone, `made`, then assigned, it's made where it's unmade.
#[derive(Debug)]
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn maybe_made_loud(made: bool) -> bool {
    let mut d;
    let mut at = 0;
    if made {
        d = Loud(1);
        at = &raw const d as usize;
    }
    d = Loud(2);
    !made || at == &raw const d as usize
}

fn address_of(n: &N) -> usize {
    n as *const N as usize
}

/// A parameter, made where it's given: moved out on one way, then
/// assigned again, it's written over.
fn given_and_moved(mut n: N, many: bool) -> (bool, N) {
    let at = &raw const n as usize;
    if many {
        let gone = n;
        println!("{gone:?}");
    }
    n = N(26, 27);
    (at == &raw const n as usize, n)
}

fn main() {
    // A move into another local.
    let m = N(1, 2);
    let at = &raw const m as usize;
    let kept = m;
    println!("moved: {} {kept:?}", at == &raw const kept as usize);

    // A move, then the local assigned again.
    let mut r = N(3, 4);
    let at = &raw const r as usize;
    let taken = r;
    r = N(5, 6);
    println!(
        "reassigned: {} {} {taken:?} {r:?}",
        at == &raw const r as usize,
        at == &raw const taken as usize
    );

    // A move into a function's parameter.
    let p = N(7, 8);
    let at = &raw const p as usize;
    let (there, second) = address_in(p);
    println!("given: {} {second}", at == there);

    // A field whose own address is taken, moved out with its struct, then
    // the struct assigned again: the moved one's field keeps its value.
    let mut o = Outer {
        inner: N(9, 10),
        tag: 11,
    };
    let inner_at = &raw const o.inner as usize;
    let old = o;
    o = Outer {
        inner: N(12, 13),
        tag: 14,
    };
    println!(
        "nested: {} {} {old:?} {o:?}",
        inner_at == &raw const o.inner as usize,
        inner_at == &raw const old.inner as usize
    );

    // In a loop: each round moves out and assigns again.
    let mut l = N(0, 0);
    let at = &raw const l as usize;
    let mut seen = Vec::new();
    for i in 1..4u8 {
        let was = l;
        l = N(i, u32::from(i) * 10);
        seen.push(was);
    }
    println!("looped: {} {seen:?} {l:?}", at == &raw const l as usize);

    // An array of them, each copied.
    let a = [N(14, 15), N(16, 17)];
    let at = &raw const a as usize;
    let a_first = address_of(&a[0]);
    let b = a;
    println!(
        "array: {} {} {b:?}",
        at == &raw const b as usize,
        a_first == address_of(&b[0])
    );

    // Moved out on one way alone, then assigned again: its object still.
    let many = std::env::args().count() > 5;
    let mut c = N(18, 19);
    let at = &raw const c as usize;
    if many {
        let gone = c;
        println!("{gone:?}");
    }
    c = N(20, 21);
    println!("maybe moved: {} {c:?}", at == &raw const c as usize);

    // Made on one way alone, then assigned: made where it's unmade.
    for made in [true, false] {
        let (same, d) = maybe_made(made);
        println!("maybe made: {same} {d:?}");
    }

    let (same, n) = given_and_moved(N(28, 29), many);
    println!("parameter: {same} {n:?}");

    for made in [true, false] {
        println!("maybe made, dropped: {}", maybe_made_loud(made));
    }

    // A `Box`'s contents, moved out and written again: its own, as a
    // local's (#1484).
    let mut boxed = Box::new(N(40, 41));
    let at = &raw const *boxed as usize;
    let out = Box::new(*boxed);
    *boxed = N(42, 43);
    println!(
        "boxed: {} {} {out:?} {boxed:?}",
        at == &raw const *boxed as usize,
        at == &raw const *out as usize
    );
}
