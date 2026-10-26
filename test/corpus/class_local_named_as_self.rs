// A class's method whose `let` is named as its `self` is, `let point` of a
// `Point`'s: its `self` is `this` from the start, so the `let` is a
// variable of its own, `const point = new Point(..)`, never `this = ..`
// (ADR 0761). A plain method's `self` is a parameter, `point`, which a
// `let` read no more after it takes for its own (ADR 0650).

#[cfg_attr(rust_js, rust_js::class)]
pub struct Point {
    x: f64,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Point {
    pub fn new(x: f64) -> Self {
        Point { x }
    }

    pub fn moved(&self, by: f64) -> f64 {
        let point = Point::new(self.x + by);
        point.x
    }

    pub fn either(&self, other: Option<Point>) -> f64 {
        let point = other.unwrap_or_else(|| Point::new(self.x * 2.0));
        point.x + self.x
    }
}

pub struct Plain {
    x: f64,
}

impl Plain {
    pub fn moved(&self, by: f64) -> f64 {
        let plain = Plain { x: self.x + by };
        plain.x
    }
}

fn main() {
    let point = Point::new(1.5);
    println!("{} {}", point.moved(2.0), point.x);
    println!("{} {}", point.either(None), point.either(Some(Point::new(10.0))));
    let plain = Plain { x: 3.0 };
    println!("{} {}", plain.moved(1.0), plain.x);
}
