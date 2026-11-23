// A `cycle()` of a `Vec`'s owned items, made in place, whose clone of what
// it's given panics: what it was given is dropped as it unwinds, as
// `Cycle::new` drops it (#1585, #1979).
use std::panic::catch_unwind;

struct P(u32);

impl Clone for P {
    fn clone(&self) -> P {
        if self.0 == 2 {
            panic!("clone of 2");
        }
        P(self.0)
    }
}

impl Drop for P {
    fn drop(&mut self) {
        println!("drop p {}", self.0);
    }
}

fn main() {
    let r = catch_unwind(|| {
        let n = vec![P(1), P(2), P(3)].into_iter().cycle().take(2).map(|p| p.0).sum::<u32>();
        println!("sum {n}");
    });
    println!("{}", r.is_err());
}
