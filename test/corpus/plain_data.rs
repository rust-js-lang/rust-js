// Plain data, numbers, `bool`s and `char`s in tuples, arrays and structs,
// is its bytes as rustc lays them out: `transmute` reads them as another
// type's, little-endian, and `zeroed` makes them all zero (ADR 0408).
use std::mem::{self, MaybeUninit};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Halves {
    low: u16,
    high: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Padded {
    tag: u8,
    value: u32,
}

#[repr(C, align(4))]
#[derive(Debug)]
struct Aligned {
    low: u16,
    high: u16,
}

// `halves` is at 4, after `Padded`'s padding.
#[repr(C)]
#[derive(Debug)]
struct Split {
    tag: u8,
    halves: Aligned,
}

#[derive(Debug, Default, PartialEq)]
struct Point {
    x: f64,
    y: i64,
    on: bool,
}

#[derive(Debug)]
struct Nothing;

fn bits_of(x: f32) -> u32 {
    unsafe { mem::transmute(x) }
}

fn longest<'a>(words: &'a [&'a str]) -> &'static str {
    let mut best = "";
    for word in words {
        if word.len() > best.len() {
            // Only its lifetime changes.
            best = unsafe { mem::transmute::<&'a str, &'static str>(word) };
        }
    }
    best
}

fn first_len<'a>(words: &'a [&'a str]) -> usize {
    let p: *const [&'a str] = words;
    // A pointer to a slice whose items' lifetime alone changes: the pointer.
    let q = unsafe { mem::transmute::<*const [&'a str], *const [&'static str]>(p) };
    let all: &[&str] = unsafe { &*q };
    all[0].len()
}

fn main() {
    unsafe {
        // The part at the same offset, of the same size.
        println!("{}", mem::transmute::<[isize; 1], isize>([7]));
        println!("{}", mem::transmute::<i32, u32>(-1));
        println!("{} {}", bits_of(1.0), mem::transmute::<u32, f32>(0x4049_0fdb));
        println!("{}", mem::transmute::<u64, f64>(0x4000_0000_0000_0000));
        println!("{:?}", mem::transmute::<Halves, [u16; 2]>(Halves { low: 1, high: 2 }));
        println!("{}", mem::transmute::<u8, bool>(1));
        println!("{:?}", mem::transmute::<u32, char>(0x41));
        println!("{:?}", mem::transmute::<(), Nothing>(()));

        // Read from the bytes of the whole.
        println!("{:#x}", mem::transmute::<[u8; 4], u32>([0x78, 0x56, 0x34, 0x12]));
        println!("{:?}", mem::transmute::<u32, [u8; 4]>(0x1234_5678));
        println!("{}", mem::transmute::<(u8, u8), u16>((1, 1)));
        println!("{:#x}", mem::transmute::<Halves, u32>(Halves { low: 0xbeef, high: 0xdead }));
        println!("{:?}", mem::transmute::<u32, Halves>(0xdead_beef));
        println!("{:?}", mem::transmute::<i64, [i32; 2]>(-2));
        println!("{:?}", mem::transmute::<[u32; 2], u64>([1, 1]));
        // Its padding, bytes 1 to 3, is no part's.
        println!("{:?}", mem::transmute::<[u8; 8], Padded>([9, 7, 7, 7, 4, 3, 2, 1]));
        println!("{:?}", mem::transmute::<Padded, Split>(Padded { tag: 9, value: 0x0102_0304 }));
        let split = Split { tag: 9, halves: Aligned { low: 0x0304, high: 0x0102 } };
        println!("{:?}", mem::transmute::<Split, Padded>(split));
        println!("{:?}", mem::transmute::<f64, [u16; 4]>(1.5));
        println!("{}", mem::transmute::<[u8; 4], f32>([0, 0, 0x80, 0x3f]));
        println!("{:?}", mem::transmute::<u128, [u64; 2]>(1 << 64 | 3));
        let x = 0x0102_0304u32;
        let swapped = mem::transmute::<[u16; 2], u32>(mem::transmute::<u32, [u16; 2]>(x));
        println!("{:#x}", swapped);

        // All zero.
        let n: u32 = mem::zeroed();
        let pair: (u8, bool) = mem::zeroed();
        let items: [u16; 3] = mem::zeroed();
        let point: Point = mem::zeroed();
        let c: char = mem::zeroed();
        let unit: () = mem::zeroed();
        println!("{n} {pair:?} {items:?} {point:?} {:?} {unit:?}", c as u32);
        println!("{}", point == Point::default());
        let halves = MaybeUninit::<Halves>::zeroed().assume_init();
        println!("{halves:?}");
    }
    println!("{}", longest(&["a", "abc", "ab"]));
    println!("{}", first_len(&["four", "x"]));
}
