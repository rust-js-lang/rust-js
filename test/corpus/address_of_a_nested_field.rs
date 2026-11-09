//@ run-pass
// The address of a field of a struct that's an array's item or another
// struct's field is in the allocation its place starts from, at the
// place's byte offset there, as rustc lays it out, though no pointer to
// the item or the field was made first (#1089, ADR 0562): not the inner
// object's own allocation.
struct P {
    a: u8,
    b: u32,
}

struct Outer {
    x: u16,
    inner: P,
}

fn main() {
    let items = [P { a: 1, b: 2 }, P { a: 3, b: 4 }, P { a: 5, b: 6 }];
    let first = &raw const items as usize;
    println!(
        "{} {}",
        &raw const items[1].b as usize - first,
        &raw const items[2].a as usize - first
    );
    // An index that's a variable's.
    let mut seen = Vec::new();
    for i in 0..items.len() {
        seen.push(&raw const items[i].b as usize - first);
    }
    println!("{seen:?}");
    let o = Outer {
        x: 1,
        inner: P { a: 2, b: 3 },
    };
    let at = &raw const o as usize;
    println!(
        "{} {} {}",
        &raw const o.inner.b as usize - at,
        &raw const o.inner.a as usize - at,
        &raw const o.x as usize - at
    );
    // Written through such a pointer, the field itself.
    let mut grid = [
        Outer {
            x: 0,
            inner: P { a: 0, b: 0 },
        },
        Outer {
            x: 0,
            inner: P { a: 0, b: 7 },
        },
    ];
    let p = &raw mut grid[1].inner.b;
    unsafe { *p += 1 };
    println!("{} {}", grid[1].inner.b, items[0].a as u32 + o.inner.b + o.x as u32);
}
