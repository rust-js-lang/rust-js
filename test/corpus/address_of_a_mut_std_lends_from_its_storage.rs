//@ native: wasm32
// A `&mut` std lends from inside its receiver's storage, made a raw pointer
// in generic code, is in the receiver's allocation at the offset rustc's
// layout gives it (#1474): a `Result`'s `Ok` or `Err` payload, a
// `OnceCell`'s value past its `Option`'s tag or in its niche, a `LazyCell`'s
// past its state's, a `Peekable`'s item past its two `Option`s', and a
// heap's top in its buffer. JS keeps each in a property of the receiver's
// object, `r._0`, `cell.value`, `peekable.peeked`, a handle on which,
// `$ptr(r, "_0", 4)`, says where. Each payload a number, a `bool`, a
// `NonZeroU32` or a reference, whose `Option`s are tagged or use a niche.
// A `Result` whose variant changes: the old pointer's address, and a new
// `as_mut`'s at the new variant's offset; a pointer read through after
// the change is undefined behaviour, none to compare. Run natively on
// wasm32, whose layout is rust-js's.
use std::cell::{LazyCell, OnceCell};
use std::collections::BinaryHeap;
use std::num::NonZeroU32;

macro_rules! sweep {
    ($name:ident, $t:ty, $e:ty, $v:expr, $w:expr) => {
        mod $name {
            use super::*;

            fn ok_ptr<T, E>(r: &mut Result<T, E>) -> *mut T {
                match r.as_mut() {
                    Ok(x) => x as *mut T,
                    Err(_) => std::ptr::null_mut(),
                }
            }

            fn err_ptr<T, E>(r: &mut Result<T, E>) -> *mut E {
                match r.as_mut() {
                    Ok(_) => std::ptr::null_mut(),
                    Err(e) => e as *mut E,
                }
            }

            fn once_ptr<T>(c: &mut OnceCell<T>) -> *mut T {
                match c.get_mut() {
                    Some(x) => x as *mut T,
                    None => std::ptr::null_mut(),
                }
            }

            fn lazy_ptr<T, F: FnOnce() -> T>(l: &mut LazyCell<T, F>) -> *mut T {
                LazyCell::force_mut(l) as *mut T
            }

            fn peek_ptr<I: Iterator>(p: &mut std::iter::Peekable<I>) -> *mut I::Item {
                match p.peek_mut() {
                    Some(x) => x as *mut I::Item,
                    None => std::ptr::null_mut(),
                }
            }

            pub fn run() {
                let mut r: Result<$t, $e> = Ok($v);
                let base = &raw const r as usize;
                let ok = ok_ptr(&mut r) as usize - base;
                // A new `as_mut` once its variant's changed, at the new
                // variant's offset, from where `r` is now: a local assigned
                // again keeps its address in Rust, not yet in JS (#1484).
                r = Err($w);
                let base = &raw const r as usize;
                let err = err_ptr(&mut r) as usize - base;
                println!("{} result ok {ok} err {err}", stringify!($name));
                let mut c: OnceCell<$t> = OnceCell::new();
                let _ = c.set($v);
                let base = &raw const c as usize;
                println!("{} once {}", stringify!($name), once_ptr(&mut c) as usize - base);
                let mut l = LazyCell::new(|| $v);
                let base = &raw const l as usize;
                println!("{} lazy {}", stringify!($name), lazy_ptr(&mut l) as usize - base);
                let mut p = vec![$v].into_iter().peekable();
                let base = &raw const p as usize;
                println!("{} peek {}", stringify!($name), peek_ptr(&mut p) as usize - base);
            }
        }
    };
}

static SEVEN: u32 = 7;

sweep!(of_u8, u8, u16, 5, 6);
sweep!(of_u16, u16, u8, 5, 6);
sweep!(of_u64, u64, u8, 5, 6);
sweep!(of_bool, bool, u32, true, 6);
sweep!(of_nonzero, NonZeroU32, u8, NonZeroU32::MIN, 6);
sweep!(of_ref, &'static u32, u64, &SEVEN, 6);

fn heap_ptr<T: Ord>(h: &mut BinaryHeap<T>) -> *mut T {
    let mut top = h.peek_mut().unwrap();
    &mut *top as *mut T
}

fn main() {
    of_u8::run();
    of_u16::run();
    of_u64::run();
    of_bool::run();
    of_nonzero::run();
    of_ref::run();
    let mut h = BinaryHeap::from(vec![3u32, 9, 4]);
    let base = h.as_slice().as_ptr() as usize;
    println!("heap {}", heap_ptr(&mut h) as usize - base);
}
