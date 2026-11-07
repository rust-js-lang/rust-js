//@ run-abort
// A closure's captures are dropped in turn, as a struct's fields are. When
// one panics, the captures after it drop as that panic unwinds, and one of
// those that panics too aborts there (#865, #887). Here the closure is
// dropped as the fold that took it ends, by the drop the closure carries.
// So `drop 1`, capture 2 panics, capture 3 panics, then the abort, and
// there's no `drop 4`.
struct Part(u32);

impl Drop for Part {
    fn drop(&mut self) {
        if self.0 == 2 || self.0 == 3 {
            panic!("part {} panics", self.0);
        }
        println!("drop {}", self.0);
    }
}

fn main() {
    let r = std::panic::catch_unwind(|| {
        let (a, b, c, d) = (Part(1), Part(2), Part(3), Part(4));
        let total = vec![1u32, 2].into_iter().fold(0, move |sum, x| {
            let (a, b, c, d) = (&a, &b, &c, &d);
            sum + x + a.0 + b.0 + c.0 + d.0
        });
        println!("total {total}");
    });
    println!("never {}", r.is_err());
}
