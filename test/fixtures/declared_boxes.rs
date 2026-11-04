pub mod shape {
    pub trait Shape {
        fn area(&self) -> f64;
    }

    pub struct Square {
        pub side: f64,
    }

    impl Shape for Square {
        fn area(&self) -> f64 {
            self.side * self.side
        }
    }
}

use shape::{Shape, Square};

pub fn bump(n: &mut u32) {
    *n += 1;
}

pub fn set<T>(slot: &mut T, value: T) {
    *slot = value;
}

pub fn shout(text: &mut String) {
    text.push('!');
}

pub fn fill(slot: &mut Option<u32>) {
    *slot = Some(slot.unwrap_or(0) + 1);
}

pub fn largest(items: &mut [u32]) -> &mut u32 {
    items.iter_mut().max().unwrap()
}

pub fn apply(f: &mut dyn FnMut(&mut u32), n: &mut u32) {
    f(n);
}

pub fn call<F: FnMut()>(f: &mut F) {
    f();
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Level {
    Low,
    High,
}

impl Level {
    pub fn raise(&mut self) {
        *self = Level::High;
    }
}

pub struct Counter {
    pub count: u32,
}

impl Counter {
    pub fn each<F: FnMut(u32)>(&self, f: &mut F) {
        f(self.count);
    }
}

pub fn tick(counter: &mut Counter) {
    counter.count += 1;
}

pub fn grow(items: &mut Vec<u32>) {
    items.push(items.len() as u32);
}

pub fn square(side: f64) -> Box<dyn Shape> {
    Box::new(Square { side })
}

pub fn area_of(shape: &dyn Shape) -> f64 {
    shape.area()
}

pub fn total(shapes: Vec<Box<dyn Shape>>) -> f64 {
    shapes.iter().map(|shape| shape.area()).sum()
}

pub trait Steps {
    fn step(&mut self);
}

impl Steps for Counter {
    fn step(&mut self) {
        self.count += 2;
    }
}

pub fn stepper() -> Box<dyn Steps> {
    Box::new(Counter { count: 0 })
}
