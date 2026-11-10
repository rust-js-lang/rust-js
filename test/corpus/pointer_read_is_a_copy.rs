// What a pointer's read gives of a `Copy` value JS holds as an object, an
// array or a struct, is a copy, as `*p` is: a later write into its place
// through the pointer doesn't show in it (#1265). It was the object in
// place, `[9, 4]` where Rust's is `[3, 4]`. `replace` and `swap` move the
// value out, which the place no longer holds, and `MaybeUninit`'s
// `assume_init_read` reads it as `read` does.
use std::mem::MaybeUninit;
use std::ptr::{self, NonNull};

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let mut v = [[1u8, 2], [3, 4]];
    let p = v.as_mut_ptr();
    unsafe {
        let read = p.add(1).read();
        let unaligned = p.read_unaligned();
        let volatile = ptr::read_volatile(p.add(1));
        let path = ptr::read(p);
        (*p.add(1))[0] = 9;
        (*p)[1] = 8;
        println!("{read:?} {unaligned:?} {volatile:?} {path:?}");

        let old = ptr::replace(p, [5, 5]);
        (*p)[0] = 6;
        println!("{old:?}");
        ptr::swap(p, p.add(1));
        (*p)[1] = 7;
        println!("{v:?}");
    }

    let mut point = Point { x: 1, y: 2 };
    let q = NonNull::from(&mut point);
    unsafe {
        let before = q.read();
        (*q.as_ptr()).x = 10;
        println!("{before:?} {point:?}");
    }

    let mut held = MaybeUninit::new([1u32, 2]);
    unsafe {
        let first = held.assume_init_read();
        (*held.as_mut_ptr())[0] = 3;
        println!("{first:?} {:?}", held.assume_init());
    }

    // Only written whole and read so, it's its value (#1258).
    let mut once = MaybeUninit::<[u32; 2]>::uninit();
    once.write([4, 5]);
    let whole = unsafe { once.assume_init_read() };
    println!("{whole:?}");
}
