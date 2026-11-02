//! A consumer that gives the library's `Small` an item it drops, and one
//! of no size: it compiles the library's methods itself, from its MIR, whose
//! `size_of::<A::Item>()` its `Array` dictionary gives, the library's (#476).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let mut small: dep::Small<[Loud; 2]> = dep::Small::default();
    for n in 1..4 {
        small.push(Loud(n));
        println!("{n}: spilled {}", small.spilled());
    }
    println!("truncated, spilled {}", small.truncate(1));
    let mut units: dep::Small<[(); 2]> = dep::Small::new();
    for _ in 0..3 {
        units.push(());
    }
    println!("units spilled {}", units.spilled());
}
