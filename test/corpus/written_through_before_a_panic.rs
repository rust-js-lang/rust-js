// A write through a `&mut` before the call it's given panics (#487): what
// was written is the place's as the call unwinds, as Rust's is. A variable
// a closure captures, or one a `&mut` parameter reaches, given on, holds it
// once the panic is caught; one whose drop the unwinding runs drops what was
// written. A variable of the body's own that nothing reads as it unwinds
// is copied back once the call returns alone, as is any whose call can't
// panic.

use std::panic::{self, AssertUnwindSafe};

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

fn set_then_panic(slot: &mut Option<u32>) -> u32 {
    *slot = Some(1);
    panic!("no value")
}

fn given_on(slot: &mut Option<u32>) -> u32 {
    set_then_panic(slot) + 1
}

fn replace_then_panic(slot: &mut Option<Noisy>) {
    *slot = Some(Noisy("written"));
    panic!("no noise")
}

fn dropped_as_it_unwinds() {
    let mut slot = Some(Noisy("first"));
    replace_then_panic(&mut slot);
    println!("never {}", slot.is_some());
}

fn bump(n: &mut u32) {
    *n += 1;
}

fn own_variable(fail: bool) -> u32 {
    let mut n = 1;
    bump(&mut n);
    let mut slot = None;
    if fail {
        set_then_panic(&mut slot);
    }
    n + slot.unwrap_or(0)
}

fn main() {
    let mut slot = None;
    let caught = panic::catch_unwind(AssertUnwindSafe(|| set_then_panic(&mut slot)));
    println!("{} {:?}", caught.is_err(), slot);
    let mut again = None;
    let caught = panic::catch_unwind(AssertUnwindSafe(|| given_on(&mut again)));
    println!("{} {:?}", caught.is_err(), again);
    println!("{}", panic::catch_unwind(dropped_as_it_unwinds).is_err());
    println!("{:?}", panic::catch_unwind(|| own_variable(false)).ok());
    println!("{:?}", panic::catch_unwind(|| own_variable(true)).ok());
    let mut count = 0;
    let mut add = || bump(&mut count);
    add();
    add();
    println!("{count}");
}
