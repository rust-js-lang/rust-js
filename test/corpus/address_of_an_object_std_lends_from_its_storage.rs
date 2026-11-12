//@ native: wasm32
// An object std lends from inside its receiver's storage, shared, is in the
// receiver's allocation at the offset rustc's layout gives it (#1475): a
// `OnceCell`'s value, `get`'s and `get_or_init`'s, a `Peekable`'s peeked
// item, `peek`'s, and a `Result`'s `Ok` or `Err` payload, `as_ref`'s; and each
// lent `&mut`, `get_mut`, `peek_mut`, `as_mut`, and a `LazyCell`'s. JS keeps
// each as an object in a property of the receiver's, `once.value`,
// `it.peeked`, `r._0`, which the call records as stored there once it's
// made, as `get_or_init` and `peek` fill it as they're called. A struct, a
// tuple and an array each payload, and an `Option` of a struct, whose object
// is past its tag, a `Result`'s; a `Result` whose variants keep theirs at
// different offsets.
// Run natively on wasm32, whose layout is rust-js's.
use std::cell::{LazyCell, OnceCell};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct P {
    a: u8,
    b: u32,
}

macro_rules! sweep {
    ($name:literal, $t:ty, $v:expr) => {{
        let once: OnceCell<$t> = OnceCell::new();
        let _ = once.set($v);
        let base = &raw const once as usize;
        println!("{} get {}", $name, once.get().unwrap() as *const $t as usize - base);
        let filled: OnceCell<$t> = OnceCell::new();
        let base = &raw const filled as usize;
        println!(
            "{} get_or_init {}",
            $name,
            filled.get_or_init(|| $v) as *const $t as usize - base
        );
        let mut it = Vec::<$t>::from([$v]).into_iter().peekable();
        let base = &raw const it as usize;
        println!("{} peek {}", $name, it.peek().unwrap() as *const $t as usize - base);
        let ok: Result<$t, u8> = Ok($v);
        let base = &raw const ok as usize;
        println!("{} ok {}", $name, ok.as_ref().unwrap() as *const $t as usize - base);
        let err: Result<u8, $t> = Err($v);
        let base = &raw const err as usize;
        println!(
            "{} err {}",
            $name,
            err.as_ref().unwrap_err() as *const $t as usize - base
        );
        // Lent `&mut`, which JS gives as the object too, and a `LazyCell`'s.
        let mut once: OnceCell<$t> = OnceCell::new();
        let _ = once.set($v);
        let base = &raw const once as usize;
        println!(
            "{} get_mut {}",
            $name,
            once.get_mut().unwrap() as *mut $t as usize - base
        );
        let mut lazy = LazyCell::new(|| -> $t { $v });
        let base = &raw const lazy as usize;
        println!(
            "{} force {}",
            $name,
            &*LazyCell::force(&lazy) as *const $t as usize - base
        );
        println!(
            "{} force_mut {}",
            $name,
            LazyCell::force_mut(&mut lazy) as *mut $t as usize - base
        );
        let mut it = Vec::<$t>::from([$v]).into_iter().peekable();
        let base = &raw const it as usize;
        println!(
            "{} peek_mut {}",
            $name,
            it.peek_mut().unwrap() as *mut $t as usize - base
        );
        let mut err: Result<u8, $t> = Err($v);
        let base = &raw const err as usize;
        println!(
            "{} err_mut {}",
            $name,
            err.as_mut().unwrap_err() as *mut $t as usize - base
        );
    }};
}

fn main() {
    sweep!("struct", P, P { a: 1, b: 2 });
    sweep!("tuple", (u8, u16), (3, 4));
    sweep!("array", [u16; 3], [5, 6, 7]);
    // An `Option` of a struct, whose object is past its tag, as a `Result`'s
    // payload; a `OnceCell` or a `Peekable` of one keeps it in a box,
    // refused (#1475).
    let ok: Result<Option<P>, u8> = Ok(Some(P { a: 8, b: 9 }));
    let base = &raw const ok as usize;
    println!("option ok {}", ok.as_ref().unwrap() as *const Option<P> as usize - base);
    // Variants at different offsets: the `Ok`'s at 2, the `Err`'s at 4.
    for r in [Ok::<(u8, u16), P>((1, 2)), Err(P { a: 3, b: 4 })] {
        let base = &raw const r as usize;
        match r.as_ref() {
            Ok(x) => println!("mixed ok {} {:?}", x as *const (u8, u16) as usize - base, x),
            Err(e) => println!("mixed err {} {:?}", e as *const P as usize - base, e),
        }
    }
}
