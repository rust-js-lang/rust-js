//! Each as Rust runs it.
use dep::{Counter, Reset, Slot, Source, Upto, counter, here, renewed, reset, summed, through_closure, wipe};
use std::panic::Location;

#[derive(Debug, Default)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug, Default)]
struct C {
    n: i32,
}

/// Its caller where a generic `Fn` calls it: rustc's `Fn::call` shim, not where it's defined (#2335).
fn line_of<F: Fn() -> &'static Location<'static>>(f: F) -> (u32, u32) {
    let at = f();
    (at.line(), at.column())
}

pub fn main() {
    println!("{} {}", Counter::LIMIT, Counter::new().count);
    println!("{:?} {}", Upto(2).numbers(), summed(&Upto(4)));
    let pointer: fn() -> &'static Location<'static> = here;
    println!("{} {}", pointer().line(), pointer().column());
    println!("{:?}", line_of(here));
    // Each pointer to it the one function, as Rust's are (ADR 0395).
    let again: fn() -> &'static Location<'static> = here;
    println!("{}", std::ptr::fn_addr_eq(pointer, again));
    let mut tick = counter();
    let mut other = tick.clone();
    tick();
    println!("{} {}", tick(), other());
    let mut slot = Slot { value: Loud(7) };
    renewed(&mut slot);
    println!("renewed {:?}", slot.value);
    wipe(Slot { value: Loud(8) });
    println!("wiped");
    println!("{:?} {:?}", through_closure(None::<String>), through_closure(Some(3)));
    let mut c = C { n: 5 };
    reset(&mut c);
    let r: &mut dyn Reset = &mut c;
    r.reset();
    println!("{c:?}");
}
