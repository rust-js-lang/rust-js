//@ run-pass
// A field's address is in the allocation its object is stored in, however
// the pointer to it was reached: through a reference to the object, made
// where the object is stored, passed to a function or returned from one; a
// `Vec`'s item through its iterator; an object moved to another container;
// and its address asked before its container's. Each is where native Rust
// puts it (#1113).
#[repr(C)]
struct Record {
    a: u8,
    b: u32,
    c: u16,
}

struct Held(Record);

#[repr(C)]
struct Outer {
    pad: u64,
    inner: Record,
}

#[repr(C)]
struct Two {
    x: Record,
    y: Record,
}

#[repr(C)]
struct Deep {
    pad: u32,
    two: Two,
}

fn record(n: u32) -> Record {
    Record { a: n as u8, b: n, c: n as u16 }
}

fn outer(n: u32) -> Outer {
    Outer { pad: u64::from(n), inner: record(n) }
}

fn address_of_c(r: &Record) -> usize {
    &raw const r.c as usize
}

fn inner_of(o: &Outer) -> &Record {
    &o.inner
}

fn main() {
    // A reference to an object in a tuple's field, the field's address
    // asked before the tuple's.
    let other = (Held(record(1)),);
    let r = &(other.0).0;
    let c = &raw const r.c as usize;
    let start = &raw const other as usize;
    println!("held {}", c - start);

    // Given to a function.
    let o = outer(2);
    println!("passed {}", address_of_c(&o.inner) - &raw const o as usize);

    // Returned from one.
    println!("returned {}", &raw const inner_of(&o).b as usize - &raw const o as usize);

    // A `Vec`'s items, through its iterator.
    let v = vec![record(3), record(4), record(5)];
    let mut seen = Vec::new();
    for item in &v {
        seen.push(&raw const item.c as usize - v.as_ptr() as usize);
    }
    println!("items {seen:?}");

    // Moved to another container: where it is now.
    let mut a = outer(6);
    let mut b = outer(7);
    let _first = &raw const a.inner.b as usize;
    std::mem::swap(&mut a.inner, &mut b.inner);
    let moved = &b.inner;
    println!("moved {} {}", &raw const moved.b as usize - &raw const b as usize, moved.a);

    // Two deep, a struct whose field no pointer passes through holding the
    // one whose does: where each is stored, recorded.
    let deep = Deep { pad: 1, two: Two { x: record(10), y: record(11) } };
    let two = &deep.two;
    let y = &two.y;
    println!("deep {}", &raw const y.c as usize - &raw const deep as usize);

    // An array a tuple holds, its items through references: at the
    // array's address and each one's index times its size.
    let nested = ([record(12), record(13)],);
    let one = &raw const nested.0[1] as usize;
    let (i0, i1) = (&nested.0[0], &nested.0[1]);
    println!(
        "nested {} {}",
        &raw const i1.b as usize - one,
        &raw const i0.c as usize - &raw const nested as usize
    );

    // A `Vec` a tuple holds: its items in a buffer of their own, not where
    // the `Vec` is.
    let held = (vec![record(14), record(15)],);
    let mut at = Vec::new();
    for item in &held.0 {
        at.push(&raw const item.c as usize - held.0.as_ptr() as usize);
    }
    println!("held vec {at:?}");

    // A closure's capture of an array a tuple holds.
    let wrapped = ([record(16), record(17)],);
    let captured = || &raw const wrapped.0[1].b as usize;
    println!("captured {}", captured() - &raw const wrapped.0 as usize);

    // Its own address asked first, then its container's.
    let pair = (record(8), record(9));
    let second = &pair.1;
    let at = &raw const *second as usize;
    println!("second {}", at - &raw const pair as usize);
}
