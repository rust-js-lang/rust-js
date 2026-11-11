//! A consumer giving its library's generic code the library's class and a
//! value that drops, each counted, upgraded and dropped as native Rust does.
use dep::{Chunk, Loud, cyclic, share};
use std::rc::Rc;

pub fn main() {
    let (chunk, owners) = share(Chunk::new(5));
    println!("{owners} {}", chunk.bump());
    let (loud, owners) = share(Loud(7));
    println!("{owners} {}", loud.0);
    let weak = Rc::downgrade(&chunk);
    drop(chunk);
    println!("{}", weak.upgrade().is_none());
    let (made, none) = cyclic(|_| Chunk::new(9));
    println!("{none} {}", made.bump());
    let (other, none) = cyclic(|_| Loud(11));
    println!("{none} {}", other.0);
    drop(loud);
    println!("end");
}
