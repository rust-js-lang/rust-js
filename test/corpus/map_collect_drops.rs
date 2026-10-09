//@ compile-fail: `std::convert::From::from` of a value with a destructor
// Pairs with a repeated key drop the value a later one replaces, which a
// JS `Map` made of them wouldn't: refused (ADR 0321).

use std::collections::HashMap;

struct Noisy(u8);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let made = HashMap::from([(1, Noisy(1)), (1, Noisy(2))]);
    println!("{}", made.len());
}
