//! A program that gives `dep`'s `Stack` a value it drops: it compiles
//! `dep`'s code itself too, and the dictionary has the `CAPACITY` that code
//! reads, of its MIR (ADR 0176, amended).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let mut stack: dep::Stack<Loud, 2> = dep::Stack::default();
    for n in 1..4 {
        match stack.try_push(Loud(n)) {
            Ok(()) => println!("pushed {n}"),
            Err(loud) => println!("full at {}", loud.0),
        }
    }
    println!("{}", stack.len());
    println!("{}", mid::units());
}
