//@ native: wasm32
// A raw pointer to what `Option::as_mut` lends is in the `Option`'s place,
// at its `Some` payload's offset, as rustc's layout has it: past the tag,
// or at the start of a niche (#1502). In a crate that asks an address, an
// `Option` local whose `&mut` goes to `as_mut` is a cell, and `as_mut`
// lends a handle on its place there, `$ptr(o, "value", 4)`, the same each
// call: two pointers compare equal, and a write through one is the
// `Option`'s. A payload of each width, a `bool`, a `NonZeroU32`, a
// reference and a `#[repr(C)]` struct; and an `Option` whose own address
// nothing asks, a cell all the same. Run natively on wasm32.
use std::num::NonZeroU32;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct P {
    a: u8,
    b: u32,
}

static SEVEN: u32 = 7;

macro_rules! sweep {
    ($name:literal, $t:ty, $v:expr, $w:expr) => {{
        let mut o: Option<$t> = Some($v);
        let base = &raw const o as usize;
        let at = &raw mut *o.as_mut().unwrap();
        let again = &raw mut *o.as_mut().unwrap();
        unsafe { *at = $w };
        println!("{} {} {} {:?}", $name, at as usize - base, at == again, o);
    }};
}

fn main() {
    sweep!("u8", u8, 1, 2);
    sweep!("u16", u16, 1, 2);
    sweep!("u32", u32, 1, 2);
    sweep!("u64", u64, 1, 2);
    sweep!("bool", bool, false, true);
    sweep!("nonzero", NonZeroU32, NonZeroU32::MIN, NonZeroU32::MAX);
    sweep!("ref", &'static u32, &SEVEN, &SEVEN);
    sweep!("struct", P, P { a: 1, b: 2 }, P { a: 3, b: 4 });
    // Its own address never asked: a cell for `as_mut`'s handle alone.
    let mut alone = Some(10u32);
    let one = &raw mut *alone.as_mut().unwrap();
    let two = &raw mut *alone.as_mut().unwrap();
    unsafe { *one += 1 };
    println!("alone {} {:?}", one == two, alone);
    // Of an `Option` reached through another's `as_mut`, no local's own
    // place: refused where its pointers are compared or its address
    // asked, never two pointers to one place that differ.
    let mut outer = Some(Some(20u32));
    let inner = outer.as_mut().unwrap();
    let three = inner.as_mut().unwrap() as *mut u32;
    unsafe { *three += 1 };
    println!("outer {outer:?}");
}
