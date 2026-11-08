// A `Wrapping`'s `op=` through a `&mut` an `Option` or a parameter gives,
// `*c.as_mut().unwrap() += d`: std's operator writes the value whole, in
// place where the reference is, so a copy read out before keeps what it
// was, as a `Copy` value's copies do (#1020), where it shared the object
// the write changed.
use std::num::Wrapping as W;

fn bump(w: &mut W<u32>, by: W<u32>) {
    *w += by;
}

fn main() {
    let mut c = Some(W(5u32));
    let d = Some(*c.as_mut().unwrap());
    *c.as_mut().unwrap() += *d.as_ref().unwrap() << 1;
    println!("{} {}", c.unwrap().0, d.unwrap().0);
    let mut held = W(7u32);
    let before = held;
    bump(&mut held, W(3));
    println!("{} {}", held.0, before.0);
    let mut slots = [W(1u8), W(2)];
    let first = slots[0];
    for slot in slots.iter_mut() {
        *slot *= W(10);
    }
    println!("{:?} {}", slots, first.0);
}
