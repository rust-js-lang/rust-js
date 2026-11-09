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

pub fn main() {
    println!("{} {}", Counter::LIMIT, Counter::new().count);
    println!("{:?} {}", Upto(2).numbers(), summed(&Upto(4)));
    let pointer: fn() -> &'static Location<'static> = here;
    println!("{} {}", pointer().line(), pointer().column());
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
