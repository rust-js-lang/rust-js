//! A consumer giving the library's zip items that drop, whose fold std
//! takes to the shorter side: refused at its call, as one crate's own
//! generic code is, where rust-js's zip would step its first side once
//! more, dropping what std's doesn't step (#2542).
#[derive(Clone, Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let held = vec![
        dep::Held {
            iter: vec![Loud(1), Loud(2)].into_iter(),
        },
        dep::Held {
            iter: Vec::new().into_iter(),
        },
    ];
    let made = dep::lasts(held, &[Loud(8), Loud(9)]);
    println!("{made:?}");
}
