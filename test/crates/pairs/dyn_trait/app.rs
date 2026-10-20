// A consumer of a library's `dyn Shape`, one the library made and one it
// makes of its own type (ADR 0524).
use dep::Shape;

struct Circle(u32);

impl Shape for Circle {
    fn area(&self) -> u32 {
        3 * self.0 * self.0
    }
}

pub fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![dep::square(2), dep::loud(5), Box::new(Circle(1))];
    for shape in &shapes {
        println!("{} {}", shape.area(), shape.name());
    }
    println!("total {}", dep::total(&shapes));
    drop(shapes);
    println!("end");
}
