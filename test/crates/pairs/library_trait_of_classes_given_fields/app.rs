// A consumer of a library's trait of classes' methods that give a field:
// each runs what it does before it gives it, as native Rust's (#1073).
use dep::{Gate, Gated};

pub fn main() {
    let mut gate = Gate::new(1);
    println!("{}", gate.mark());
    println!("{}", gate.later());
    *gate.counter() += 5;
    let counted = gate.counter();
    *counted *= 2;
    println!("{}", gate.kept());
    println!("{}", gate.guarded());
    let other = Gate::new(7);
    let pick = Gate::pick;
    println!("{} {}", pick(&gate, &other), gate.pick(&other));
    println!("{}", gate.limit());
}
