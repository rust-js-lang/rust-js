// A library's trait, whose `dyn` its consumer holds, calls and drops: the
// pair its library made (ADR 0524).
pub trait Shape {
    fn area(&self) -> u32;
    fn name(&self) -> String {
        format!("shape of {}", self.area())
    }
}

pub struct Square(pub u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

pub struct Loud(pub u32);

impl Shape for Loud {
    fn area(&self) -> u32 {
        self.0
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

pub fn square(side: u32) -> Box<dyn Shape> {
    Box::new(Square(side))
}

pub fn loud(n: u32) -> Box<dyn Shape> {
    Box::new(Loud(n))
}

pub fn total(shapes: &[Box<dyn Shape>]) -> u32 {
    shapes.iter().map(|s| s.area()).sum()
}
