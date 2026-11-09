//@ run-pass
// The address of a tuple's field, or a tuple struct's, is the tuple's and
// the field's byte offset, as rustc lays it out, which may reorder its
// fields: not its index times a size, as an array's item's is, though JS
// holds the tuple as an array (#1081, ADR 0562).
#[repr(C)]
struct Record(u8, u32, u16);

struct Held(Record);

fn offsets(t: &(u8, u64, u16)) -> [usize; 3] {
    let at = t as *const (u8, u64, u16) as usize;
    [
        &raw const t.0 as usize - at,
        &raw const t.1 as usize - at,
        &raw const t.2 as usize - at,
    ]
}

fn main() {
    // rustc puts the `u32` first, then the `u16`, then the `u8`.
    let plain: (u8, u32, u16) = (1, 2, 3);
    let at = &raw const plain as usize;
    println!(
        "{} {} {}",
        &raw const plain.0 as usize - at,
        &raw const plain.1 as usize - at,
        &raw const plain.2 as usize - at
    );
    // A `#[repr(C)]` tuple struct, held in another's field.
    let held = Held(Record(1, 2, 3));
    let start = &raw const held.0 as usize;
    println!(
        "{} {}",
        &raw const held.0.1 as usize - start,
        &raw const held.0.2 as usize - start
    );
    // An exposed field's address is that field again.
    let exposed = &raw const plain.2 as usize;
    println!("{}", unsafe { *std::ptr::with_exposed_provenance::<u16>(exposed) });
    // A tuple an array holds, its field's address in the array's where a
    // pointer to the item is made first.
    let items = [(1u8, 2u32), (3, 4), (5, 6)];
    let second = &raw const items[1];
    let first = &raw const items[0] as usize;
    println!(
        "{} {}",
        &raw const items[1].1 as usize - first,
        &raw const items[1].0 as usize - second as usize
    );
    // Of a tuple a reference refers to.
    println!("{:?}", offsets(&(1u8, 2u64, 3u16)));
    // A field's pointer reads and writes its own field.
    let mut pair = (5u16, 7u32);
    let p = &raw mut pair.1;
    unsafe { *p += 1 };
    println!("{:?}", pair);
}
