//! A program that catches a panic and shows, in stdout's line, a
//! library's type whose writer shows its dependency's writer that may stop
//! part-way: Rust's stdout keeps `x <!`, so it's refused (#1608, #1530).
use std::panic::catch_unwind;

use mid::Holder;

pub fn main() {
    let caught = catch_unwind(|| println!("x {:?} y", mid_holder()));
    println!(" caught {}", caught.is_err());
}

fn mid_holder() -> Holder {
    Holder(dep::Loud(3))
}
