// Trait objects shared and boxed: `Rc<dyn Shape>`s sorted, scaled into
// `Box<dyn Shape>`s, given to `T: ?Sized + Shape` as `&dyn Shape`s, kept by
// kind, and one upcast to `&dyn Debug` where it's indexed out, a call's,
// read once.

use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
trait Shape: Debug {
    fn area(&self) -> f64;
    fn scale(&self, k: f64) -> Box<dyn Shape>;
    fn kind(&self) -> &'static str {
        "shape"
    }
    fn twice(&self) -> f64
    where
        Self: Sized,
    {
        self.area() * 2.0
    }
}
#[derive(Debug, Clone)]
struct Sq(f64);
#[derive(Debug, Clone)]
struct Ci(f64);
impl Shape for Sq {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
    fn scale(&self, k: f64) -> Box<dyn Shape> {
        Box::new(Sq(self.0 * k))
    }
    fn kind(&self) -> &'static str {
        "sq"
    }
}
impl Shape for Ci {
    fn area(&self) -> f64 {
        3.0 * self.0 * self.0
    }
    fn scale(&self, k: f64) -> Box<dyn Shape> {
        Box::new(Ci(self.0 * k))
    }
}
fn biggest<T: ?Sized + Shape>(xs: &[&T]) -> f64 {
    xs.iter().map(|s| s.area()).fold(0.0, f64::max)
}
fn show(s: &dyn Debug) -> String {
    format!("<{s:?}>")
}
fn main() {
    let shapes: Vec<Rc<dyn Shape>> = vec![Rc::new(Sq(2.0)), Rc::new(Ci(1.0)), Rc::new(Sq(1.5))];
    let mut sorted = shapes.clone();
    sorted.sort_by(|a, b| a.area().partial_cmp(&b.area()).unwrap());
    println!("{:?}", sorted.iter().map(|s| (s.kind(), s.area())).collect::<Vec<_>>());
    let scaled: Vec<Box<dyn Shape>> = shapes.iter().map(|s| s.scale(2.0)).collect();
    println!("{scaled:?}");
    let refs: Vec<&dyn Shape> = scaled.iter().map(|b| b.as_ref()).collect();
    println!("{}", biggest(&refs));
    println!("{} {}", Sq(3.0).twice(), show(&*shapes[1]));
    let mut by_kind: HashMap<&str, Vec<Rc<dyn Shape>>> = HashMap::new();
    for s in &shapes {
        by_kind.entry(s.kind()).or_default().push(Rc::clone(s));
    }
    let mut keys: Vec<_> = by_kind
        .iter()
        .map(|(k, v)| (*k, v.len(), Rc::strong_count(&v[0])))
        .collect();
    keys.sort();
    println!("{keys:?}");
    let ops: HashMap<&str, Box<dyn Fn(f64, f64) -> f64>> = HashMap::from([
        ("add", Box::new(|a, b| a + b) as Box<dyn Fn(f64, f64) -> f64>),
        ("pow", Box::new(f64::powf)),
    ]);
    let mut names: Vec<_> = ops.keys().copied().collect();
    names.sort();
    println!("{:?}", names.iter().map(|n| ops[n](2.0, 3.0)).collect::<Vec<_>>());
    let d: &dyn Debug = &shapes;
    println!("{}", show(d).len());
}
