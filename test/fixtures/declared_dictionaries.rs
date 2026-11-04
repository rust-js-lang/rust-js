pub mod shape {
    /// What has an area.
    pub trait Shape {
        /// Its area.
        fn area(&self) -> f64;

        fn describe(&self) -> String {
            format!("a shape of area {}", self.area())
        }
    }

    pub struct Circle {
        pub radius: f64,
    }

    impl Shape for Circle {
        fn area(&self) -> f64 {
            std::f64::consts::PI * self.radius * self.radius
        }
    }

    pub struct Rect {
        pub width: f64,
        pub height: f64,
    }

    impl Shape for Rect {
        fn area(&self) -> f64 {
            self.width * self.height
        }

        fn describe(&self) -> String {
            format!("a {}x{} rectangle", self.width, self.height)
        }
    }
}

use shape::{Circle, Shape};
use std::fmt::Display;

pub trait Named {
    fn make(name: &str) -> Self;
    fn name(&self) -> String;
}

pub struct Tag {
    pub text: String,
}

impl Named for Tag {
    fn make(name: &str) -> Self {
        Tag { text: name.to_string() }
    }

    fn name(&self) -> String {
        self.text.clone()
    }
}

impl Shape for Tag {
    fn area(&self) -> f64 {
        0.0
    }
}

pub trait Counter {
    fn bump(&mut self) -> u32;
}

pub trait Sided {
    const SIDES: u32;
    fn sides(&self) -> u32 {
        Self::SIDES
    }
}

pub trait Labeled: Display {
    fn label(&self) -> String {
        format!("<{self}>")
    }
}

pub struct Wrapper<T>(pub T);

impl<T: Display> Shape for Wrapper<T> {
    fn area(&self) -> f64 {
        1.0
    }
}

pub trait Checked {
    #[track_caller]
    fn check(&self) -> u32;
}

impl Checked for Tag {
    fn check(&self) -> u32 {
        std::panic::Location::caller().line()
    }
}

pub trait Shows {
    fn show<T: Display>(&self, value: T) -> String;
}

impl Shows for Tag {
    fn show<T: Display>(&self, value: T) -> String {
        format!("{}{value}", self.text)
    }
}

pub trait Names {
    fn named<T: ?Sized>(&self) -> &'static str;
}

impl Names for Tag {
    fn named<T: ?Sized>(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

pub fn total_area<S: Shape>(shapes: &[S]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

pub fn described<S: Shape>(shape: &S, prefix: Option<&str>) -> String {
    format!("{}{}", prefix.unwrap_or(""), shape.describe())
}

pub fn renamed<N: Named + Shape>(named: &N, to: &str) -> String {
    format!("{} {} {}", named.name(), N::make(to).name(), named.area())
}

pub fn bumped<C: Counter>(c: &mut C) -> u32 {
    c.bump()
}

pub fn sides<S: Sided>(s: &S) -> u32 {
    s.sides()
}

pub fn labels<L: Labeled>(l: &L) -> String {
    l.label()
}

pub fn shown<T: Display>(value: T) -> String {
    format!("<{value}>")
}

pub fn checked<C: Checked>(c: &C) -> u32 {
    c.check()
}

pub fn shows<S: Shows>(s: &S) -> String {
    s.show(1)
}

pub fn names<N: Names>(n: &N) -> &'static str {
    n.named::<str>()
}

pub fn circles() -> f64 {
    total_area(&[Circle { radius: 1.0 }])
}
