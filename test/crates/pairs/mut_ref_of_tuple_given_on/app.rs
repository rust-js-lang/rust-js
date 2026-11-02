//! A consumer that gives the library's `Stack` a value it drops, so it
//! compiles `extend` itself, from the library's MIR, whose `&mut usize` of
//! a tuple is given on to the guard as it is (#476).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let mut stack: dep::Stack<Loud> = dep::Stack::default();
    stack.extend([Loud(1), Loud(2)]);
    println!("{}", stack.len());
    stack.extend(vec![Loud(3)]);
    println!("{}", stack.len());
}
