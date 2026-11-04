// What an adapter passes over is dropped as std drops it, where the drop
// panics: a `skip()` and a `step_by()` of a `Vec`'s items, collected, of
// items whose field's drop panics, though the item has no `Drop` of its
// own, drop what's left as the panic unwinds, in both builds.
use std::panic::catch_unwind;

struct Boom(u32);

impl Drop for Boom {
    fn drop(&mut self) {
        println!("drop boom {}", self.0);
        if self.0 == 3 {
            panic!("boom");
        }
    }
}

struct Pair {
    d: Boom,
}

fn main() {
    let r = catch_unwind(|| {
        let v: Vec<Pair> = (1..=6)
            .map(|n| Pair { d: Boom(n) })
            .collect::<Vec<_>>()
            .into_iter()
            .skip(1)
            .step_by(2)
            .collect();
        v.iter().map(|p| p.d.0).sum::<u32>()
    });
    println!("pairs {}", r.is_err());
}
