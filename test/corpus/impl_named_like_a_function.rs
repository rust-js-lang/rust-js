// A trait impl's name, `exprDebug` of `#[derive(Debug)]` on `Expr` and
// `circleShape` of `impl Shape for Circle`, and a function the program
// names so, `fn expr_debug` and `fn circle_shape`, are two names Rust
// keeps apart: JS names them apart too, as any two names it gives, where it
// refused the crate (#453).
pub trait Shape {
    fn area(&self) -> f64;
}

pub struct Circle {
    pub r: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.0 * self.r * self.r
    }
}

pub fn circle_shape() -> f64 {
    7.0
}

pub fn total(shapes: &[&dyn Shape]) -> f64 {
    shapes.iter().map(|s| s.area()).sum::<f64>() + circle_shape()
}


#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(i32),
    Add(Box<Expr>, Box<Expr>),
}

pub fn expr_debug(e: &Expr) -> String {
    format!("{e:?}")
}

fn expr_clone(e: &Expr) -> Expr {
    e.clone()
}

pub fn shown(d: &dyn std::fmt::Debug) -> String {
    format!("{d:?}")
}

fn main() {
    let c = Circle { r: 2.0 };
    println!("{}", total(&[&c, &c]));
    let e = Expr::Add(Box::new(Expr::Num(1)), Box::new(Expr::Num(2)));
    println!("{}", expr_debug(&e));
    println!("{}", expr_clone(&e) == e);
    println!("{}", shown(&e));
}
