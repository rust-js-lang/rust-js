// A trait's default method an impl leaves to the default, copied into the
// impl (f92c64c15): one that compares values of its associated type, `==`
// of `Self::Id`, and one that makes a value of it, `Self::Unit::default()`,
// each its impl's type's, a number, a string and the crate's struct.
use std::fmt::Display;

trait Registry {
    type Id: PartialEq + Copy;
    fn ids(&self) -> Vec<Self::Id>;
    fn has(&self, id: Self::Id) -> bool {
        self.ids().iter().any(|i| *i == id)
    }
}

struct Numbers;

impl Registry for Numbers {
    type Id = u32;
    fn ids(&self) -> Vec<u32> {
        vec![1, 4]
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Tag(char);

struct Tags;

impl Registry for Tags {
    type Id = Tag;
    fn ids(&self) -> Vec<Tag> {
        vec![Tag('a'), Tag('c')]
    }
}

trait Shape {
    type Unit: Default + Display;
    fn unit(&self) -> Self::Unit {
        Self::Unit::default()
    }
}

struct Tri;

impl Shape for Tri {
    type Unit = u8;
}

struct Named;

impl Shape for Named {
    type Unit = String;
}

fn main() {
    println!("{} {}", Numbers.has(4), Numbers.has(2));
    println!("{} {}", Tags.has(Tag('c')), Tags.has(Tag('b')));
    println!("[{}] [{}]", Tri.unit(), Named.unit());
}
