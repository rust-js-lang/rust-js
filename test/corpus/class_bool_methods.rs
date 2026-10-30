// A class's method's call of its own `bool` methods is a `bool`, as the
// same call of another value's is: `this.below(x) || this.same(x)`, no
// `!!`, as JS writes it; and so is a read of its own `bool` field,
// `this.open || this.below(x)`.

#[cfg_attr(rust_js, rust_js::class)]
pub struct Bound {
    limit: f64,
    open: bool,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Bound {
    pub fn new(limit: f64) -> Self {
        Bound { limit, open: false }
    }

    pub fn below(&self, x: f64) -> bool {
        x < self.limit
    }

    pub fn same(&self, x: f64) -> bool {
        x == self.limit
    }

    pub fn at_most(&self, x: f64) -> bool {
        self.below(x) || self.same(x)
    }

    pub fn open_or_below(&self, x: f64) -> bool {
        self.open || self.below(x)
    }

    pub fn shut(&self) -> bool {
        !self.open && self.limit > 0.0
    }
}

fn main() {
    let bound = Bound::new(3.0);
    println!(
        "{} {} {} {} {}",
        bound.at_most(2.0),
        bound.at_most(3.0),
        bound.at_most(4.0),
        bound.shut(),
        bound.open_or_below(2.0)
    );
}
