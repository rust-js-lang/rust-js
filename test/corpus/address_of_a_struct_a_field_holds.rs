// The address of a struct a tuple struct's field holds, `&raw const
// held.0`, and of its fields, `&raw const held.0.b`, are in one
// allocation: a field's is the struct's and its offset, as rustc lays it
// out; and a reference to the struct made an address is the struct's
// (ADR 0562, #1019). What the metamorphic net's `field` rewrite found of
// `pointer_addresses`.
use std::mem::{align_of, transmute};

#[repr(C)]
struct Record {
    a: u8,
    b: u32,
    c: u16,
}

struct Held(Record);

fn main() {
    let held = Held(Record { a: 1, b: 2, c: 3 });
    let at = &raw const held.0 as usize;
    println!(
        "{} {} {}",
        &raw const held.0.b as usize - at,
        &raw const held.0.c as usize - at,
        at % align_of::<Record>()
    );
    let referred: usize = unsafe { transmute(&held.0) };
    println!("{} {}", referred == at, held.0.a + held.0.c as u8);
    // A field's address asked before the struct's.
    let other = Held(Record { a: 2, b: 3, c: 4 });
    let start = &raw const other.0;
    let c = &raw const other.0.c as usize;
    println!("{}", c - start as usize);
    // And before any pointer to the struct is made.
    let last = Held(Record { a: 5, b: 6, c: 7 });
    let b = &raw const last.0.b as usize;
    println!("{}", b - &raw const last.0 as usize);
    let pair = (Record { a: 4, b: 5, c: 6 }, Record { a: 7, b: 8, c: 9 });
    let first = &raw const pair.0 as usize;
    let second = &raw const pair.1 as usize;
    println!(
        "{} {}",
        &raw const pair.1.b as usize - second,
        &raw const pair.0.c as usize - first
    );
}
