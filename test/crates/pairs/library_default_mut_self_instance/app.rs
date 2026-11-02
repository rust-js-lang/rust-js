//! A consumer that gives the library's `Stack` a value it drops: it
//! compiles the trait's provided method itself, from the library's MIR,
//! given the box its dictionary's callers give a `&mut Self`, which it gives
//! on as it is (#472); and so is a `&mut u32`'s.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let mut stack: dep::Stack<Loud> = dep::Stack::default();
    for n in 1..4 {
        match stack.try_push(Loud(n)) {
            Ok(()) => println!("pushed {n}"),
            Err(loud) => println!("full at {}", loud.0),
        }
    }
    println!("{}", stack.len());
    let mut count = 1;
    dep::bump(&mut count, Loud(9));
    println!("{count}");
}
