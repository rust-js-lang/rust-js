// A trait of classes' method that does something before it gives a field
// of `self`, `self.seen = true; &self.seen`, is no field read in place
// (rd304): its prototype's method, which runs it first (#1074); and one
// that gives a `&mut` of its field, a `&mut self` method's prototype
// method too, the impl's own, no default of its trait's.
trait Gated {
    fn mark(&mut self) -> &bool;
    fn counter(&mut self) -> &mut u32;
    fn kept(&self) -> &u32;
}

#[cfg_attr(rust_js, rust_js::class)]
struct Gate {
    seen: bool,
    n: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Gate {
    fn new() -> Self {
        Gate { seen: false, n: 1 }
    }
}

impl Gated for Gate {
    fn mark(&mut self) -> &bool {
        self.seen = true;
        &self.seen
    }

    fn counter(&mut self) -> &mut u32 {
        &mut self.n
    }

    fn kept(&self) -> &u32 {
        &self.n
    }
}

fn main() {
    let mut gate = Gate::new();
    println!("{}", gate.mark());
    *gate.counter() += 5;
    let counted = gate.counter();
    *counted *= 2;
    println!("{}", gate.kept());
}
