//! A consumer giving the library's zip numbers, which are `Copy`, and items
//! that drop, which aren't, so it compiles the library's function itself
//! (ADR 0914, #1389).
#[derive(Clone, Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let numbers = dep::lasts(vec![dep::Held { iter: 0..3 }, dep::Held { iter: 5..5 }], vec![10, 20]);
    println!("{numbers:?}");
    let held = vec![
        dep::Held {
            iter: vec![Loud(1), Loud(2)].into_iter(),
        },
        dep::Held {
            iter: Vec::new().into_iter(),
        },
    ];
    let made = dep::lasts(held, vec![Loud(8), Loud(9)]);
    println!("{made:?}");
}
