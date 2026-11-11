//@ run-fail: boom 2
// A `Box<dyn Iterator>` whose drop panics, dropping what a `Vec`'s
// `IntoIter` has left: the rest dropped as it unwinds, as std's are, and
// the panic out of the `Box`'s drop (ADR 0521, #1420).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 2 {
            panic!("boom {}", self.0);
        }
    }
}

fn items() -> Box<dyn Iterator<Item = Loud>> {
    Box::new(vec![Loud(1), Loud(2), Loud(3)].into_iter())
}

fn main() {
    let mut it = items();
    println!("first {}", it.next().map_or(0, |l| l.0));
    drop(it);
    println!("unreached");
}
