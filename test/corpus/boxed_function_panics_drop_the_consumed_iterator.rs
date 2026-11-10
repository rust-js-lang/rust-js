// An iterator with a destructor a std consumer takes by value, given a
// boxed function, `Box<dyn FnMut>`, that panics: dropped as the panic
// unwinds through the consumer, as of a closure (#1079, #1232).
use std::panic::catch_unwind;

struct Loud(u8);

impl Iterator for Loud {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        self.0 += 1;
        if self.0 < 5 { Some(self.0) } else { None }
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn checked(x: u8) -> u8 {
    if x == 3 {
        panic!("three");
    }
    x
}

fn main() {
    let folded = catch_unwind(|| {
        let f: Box<dyn FnMut(u8, u8) -> u8> = Box::new(|a, x| a + checked(x));
        Loud(0).fold(0u8, f)
    });
    println!("fold {}", folded.is_err());
    let each = catch_unwind(|| {
        let f: Box<dyn FnMut(u8)> = Box::new(|x| {
            checked(x);
        });
        Loud(0).for_each(f)
    });
    println!("for_each {}", each.is_err());
}
