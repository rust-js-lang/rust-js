//@ native: wasm32
// A `MaybeUninit<[T; N]>` written an item at a time through its pointer,
// `m.as_mut_ptr() as *mut T`, then taken whole: an array, each item
// written where it is (#1255, ADR 0417); found by the memory net (#1172).
// One only written whole and read whole is its value, `const m = [..]`,
// and one given to a function, which may write it through its pointer, or
// boxed, its slots (#1258).
use std::mem::MaybeUninit;
use std::ptr;

#[derive(Clone, Copy, Debug)]
struct Pair {
    left: i32,
    right: u8,
}

fn fill(slot: &mut MaybeUninit<[u32; 2]>) {
    let p = slot.as_mut_ptr() as *mut u32;
    unsafe {
        p.write(4);
        p.add(1).write(5);
    }
}

fn squares<const N: usize>() -> [u64; N] {
    let mut slots = MaybeUninit::<[u64; N]>::uninit();
    let first = slots.as_mut_ptr() as *mut u64;
    for i in 0..N {
        unsafe { first.add(i).write((i * i) as u64) };
    }
    unsafe { slots.assume_init() }
}

fn main() {
    let mut m = MaybeUninit::<[u32; 3]>::uninit();
    let p = m.as_mut_ptr() as *mut u32;
    unsafe {
        p.write(1);
        p.add(1).write(2);
        *p.add(2) = 3;
    }
    println!("{:?}", unsafe { m.assume_init() });

    let mut signed = MaybeUninit::<[i8; 2]>::uninit();
    let q = signed.as_mut_ptr().cast::<i8>();
    unsafe {
        q.write(-1);
        q.add(1).write(i8::MIN);
    }
    println!("{:?}", unsafe { signed.assume_init() });

    let mut pairs = MaybeUninit::<[Pair; 2]>::uninit();
    let r = pairs.as_mut_ptr() as *mut Pair;
    unsafe {
        r.write(Pair { left: -3, right: 4 });
        r.add(1).write(Pair { left: 5, right: 6 });
    }
    println!("{:?}", unsafe { pairs.assume_init() });

    // Its first item read back before the rest is written.
    let mut part = MaybeUninit::<[u16; 4]>::uninit();
    let s = part.as_mut_ptr() as *mut u16;
    unsafe { s.write(9) };
    println!("{}", unsafe { ptr::read(s) });

    println!("{:?}", squares::<4>());
    let mut whole = MaybeUninit::<[u8; 2]>::uninit();
    whole.write([7, 8]);
    println!("{:?}", unsafe { whole.assume_init() });

    let mut given = MaybeUninit::<[u32; 2]>::uninit();
    fill(&mut given);
    println!("{:?}", unsafe { given.assume_init() });
    let mut boxed = Box::new(MaybeUninit::<[u32; 2]>::uninit());
    fill(&mut boxed);
    println!("{:?}", unsafe { boxed.assume_init() });

    let mut read = MaybeUninit::<[i16; 3]>::uninit();
    let written = read.write([1, -2, 3]);
    written[1] = 20;
    println!("{:?} {:?}", unsafe { read.assume_init_ref() }, unsafe {
        read.assume_init_read()
    });
}
