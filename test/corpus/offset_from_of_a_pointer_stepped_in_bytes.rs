//@ native: wasm32
// `offset_from` of a pointer stepped in bytes and cast back to its items'
// type, `p.cast::<u8>().add(8).cast::<u64>()`, and the array's own
// pointer: how many items apart they are, not bytes (#1254, ADR 0562);
// found by the memory net (#1172).
fn index_of<T>(items: &[T], at: *const T) -> isize {
    unsafe { at.offset_from(items.as_ptr()) }
}

fn main() {
    let longs: [u64; 3] = [1, 2, 3];
    let second = unsafe { longs.as_ptr().cast::<u8>().add(8).cast::<u64>() };
    println!("{}", unsafe { second.offset_from(longs.as_ptr()) });
    println!("{}", unsafe { longs.as_ptr().offset_from(second) });
    println!("{}", unsafe { longs.as_ptr().add(2).offset_from(second) });
    println!("{}", unsafe { *second });

    let halves: [i16; 5] = [1, -2, 3, -4, 5];
    let fourth = unsafe { halves.as_ptr().cast::<u8>().add(6).cast::<i16>() };
    println!("{} {}", unsafe { fourth.offset_from(halves.as_ptr()) }, unsafe { *fourth });
    let start = unsafe { halves.as_ptr().cast::<u8>().cast::<i16>() };
    println!("{}", unsafe { fourth.offset_from(start) });
    println!("{}", index_of(&halves, fourth));

    let mut words = [10u32, 20, 30, 40];
    let p = words.as_mut_ptr();
    let third = unsafe { p.cast::<u8>().add(8).cast::<u32>() };
    unsafe { *third += 1 };
    println!("{} {words:?}", unsafe { third.offset_from(p) });
}
