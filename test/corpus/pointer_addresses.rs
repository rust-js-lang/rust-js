// A pointer is a place and an address (ADR 0562): its address differs from
// another's into the same allocation by rustc's offsets, is aligned as the
// type is, round-trips through an integer, and orders; an integer
// made a pointer is one with no place, equal to another of its address.
use std::hash::{DefaultHasher, Hash, Hasher};
use std::mem::{align_of, size_of, transmute};
use std::ptr::{self, NonNull};

#[repr(C)]
struct Record {
    a: u8,
    b: u32,
    c: u16,
}

#[repr(C, packed)]
struct Packed {
    a: u8,
    b: u32,
}

// A pointer to a type parameter's value, an object as its caller gives it.
fn address_of<T>(value: &T) -> usize {
    value as *const T as usize
}

fn named() {}

fn main() {
    let items = [10u32, 20, 30, 40];
    let p = items.as_ptr();
    let second = unsafe { p.add(1) };
    let base = p as usize;
    println!(
        "{} {} {} {}",
        second as usize - base,
        unsafe { p.add(3) } as usize - base,
        base % align_of::<u32>(),
        unsafe { second.offset_from(p) }
    );

    let record = Record { a: 1, b: 2, c: 3 };
    let at = &raw const record as usize;
    println!(
        "{} {} {}",
        &raw const record.b as usize - at,
        &raw const record.c as usize - at,
        at % align_of::<Record>()
    );
    let two = [Packed { a: 1, b: 2 }, Packed { a: 3, b: 4 }];
    let stride = &raw const two[1] as usize - &raw const two[0] as usize;
    let through = &two[1] as *const Packed;
    println!(
        "{} {} {} {} {}",
        stride,
        size_of::<Packed>(),
        two[1].a,
        through == &raw const two[1],
        through as usize == &raw const two[1] as usize
    );

    // Round trips: an address made a pointer again is the pointer.
    let again = base as *const u32;
    let exposed = ptr::with_exposed_provenance::<u32>(p.expose_provenance());
    println!(
        "{} {} {} {}",
        again == p,
        exposed == p,
        unsafe { *again.add(2) },
        p.addr() == base
    );
    let n: usize = unsafe { transmute(p) };
    println!("{} {}", n == base, n as *const u32 == p);

    // An address with no place: equal to another of it, never null, not read.
    let one = 1 as *const u8;
    let other = 1 as *const u8;
    let none = 0 as *const u8;
    println!("{} {} {} {}", one == other, one as usize, one.is_null(), none.is_null());
    println!(
        "{} {}",
        NonNull::<u32>::dangling().as_ptr() as usize,
        ptr::dangling::<u64>() as usize
    );

    // Moved along, by items or by bytes, and back; aligned, or a byte off.
    println!(
        "{} {} {} {} {}",
        p.wrapping_byte_add(4) == second,
        second.map_addr(|a| a - 4) == p,
        p.with_addr(base + 8) == unsafe { p.add(2) },
        p.is_aligned(),
        p.wrapping_byte_add(1).is_aligned()
    );
    let at_record = &raw const record;
    println!(
        "{} {}",
        at_record.wrapping_byte_add(4).is_aligned(),
        at_record.wrapping_byte_add(2).is_aligned()
    );

    // Ordered along the allocation, and shown.
    let mut pointers = [unsafe { p.add(2) }, p, second];
    pointers.sort();
    println!(
        "{} {} {}",
        pointers[0] == p,
        pointers[2] == unsafe { p.add(2) },
        p < second
    );
    let shown = format!("{p:p}");
    println!(
        "{} {} {}",
        shown == format!("{p:p}"),
        shown.starts_with("0x"),
        format!("{none:?}")
    );

    // A boxed value's, aligned as its type is; two allocations' differ.
    let boxed = Box::into_raw(Box::new(Record { a: 4, b: 5, c: 6 }));
    println!("{} {}", boxed as usize % align_of::<Record>(), unsafe { (*boxed).b });
    println!("{} {}", boxed as usize != at, address_of(&record) == at);
    drop(unsafe { Box::from_raw(boxed) });

    // Hashed by address: one place hashes the same, two differently.
    let hashed = |pointer: *const u32| {
        let mut hasher = DefaultHasher::new();
        pointer.hash(&mut hasher);
        hasher.finish()
    };
    println!("{} {}", hashed(p) == hashed(again), hashed(p) == hashed(second));

    // A reference's address is its object's; a zero-sized value's its
    // alignment; a function's is the function's.
    let referred: usize = unsafe { transmute(&record) };
    let unit: &() = &();
    let f = named as fn();
    println!(
        "{} {} {} {}",
        referred == at,
        unit as *const () as usize,
        f as *const u8 == f as *const u8,
        f as *const u8 as usize % 2
    );
}
