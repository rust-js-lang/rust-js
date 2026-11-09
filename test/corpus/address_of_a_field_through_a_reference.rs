//@ run-pass
// The address of a field of a struct reached through a reference, `&raw
// const r.b` of `let r = &held.0;`, whose place starts at what the
// reference refers to: in the allocation that holds that struct, where a
// pointer to it was made or is made after (#1019, #1081), as rustc lays
// it out. What a reference refers to isn't known where the pointer is
// made, so the struct is found as the item or the field a pointer to it
// was made of (ADR 0562).
#[repr(C)]
struct Record {
    a: u8,
    b: u32,
    c: u16,
}

#[repr(C)]
struct Numbered(u8, u32, u16);

struct Held(Record);

struct Pair(Numbered, Numbered);

fn main() {
    // A field of a tuple struct's field, its struct's pointer made first.
    let held = Held(Record { a: 1, b: 2, c: 3 });
    let at = &raw const held.0 as usize;
    let r = &held.0;
    println!("{} {}", &raw const r.b as usize - at, &raw const r.c as usize - at);
    // A field's address asked before the struct's.
    let other = Held(Record { a: 2, b: 3, c: 4 });
    let r = &other.0;
    let c = &raw const r.c as usize;
    let start = &raw const other.0 as usize;
    println!("{}", c - start);
    // Of a tuple struct's second field, a tuple struct of its own.
    let pair = Pair(Numbered(4, 5, 6), Numbered(7, 8, 9));
    let second = &raw const pair.1 as usize;
    let first = &raw const pair.0 as usize;
    let (r0, r1) = (&pair.0, &pair.1);
    println!(
        "{} {} {}",
        &raw const r1.1 as usize - second,
        &raw const r0.2 as usize - first,
        second - first
    );
    // Of an array's items, the array's pointers made first.
    let items = [Record { a: 1, b: 2, c: 3 }, Record { a: 4, b: 5, c: 6 }];
    let one = &raw const items[1] as usize;
    let zero = &raw const items[0] as usize;
    let (i0, i1) = (&items[0], &items[1]);
    println!(
        "{} {} {}",
        &raw const i1.b as usize - one,
        &raw const i0.c as usize - zero,
        one - zero
    );
    println!("{}", held.0.a as u32 + other.0.b + r1.0 as u32 + items[1].b);
}
