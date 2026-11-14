// A private struct whose field holds an `Rc` of its type parameter,
// `Entries<T>`'s `Vec<Rc<T>>`, beside an untagged enum one of whose
// variants is an `Rc` of a struct, and an `Rc<u8>` compared by
// `Rc::ptr_eq`, which counts it (#1816). No other crate can make an
// `Entries`, so its `Rc<T>` is what the crate's own bodies make, each `T`
// its callers give, built as a program or as a library: the `Rc<Shape>`
// isn't counted, and the enum's variants are told apart.

use std::rc::Rc;

pub struct Shape {
    pub sides: u32,
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Source {
    Shape(Rc<Shape>),
    Make(Box<dyn Fn() -> Rc<Shape>>),
}

impl Source {
    pub fn sides(&self) -> u32 {
        match self {
            Source::Shape(shape) => shape.sides,
            Source::Make(make) => make().sides,
        }
    }
}

struct Entries<T> {
    all: Vec<Rc<T>>,
}

/// The first of `items` that's the same allocation as `of`, by its index.
pub fn position_of<T>(items: &[Rc<T>], of: &Rc<T>) -> Option<usize> {
    let entries = Entries { all: items.to_vec() };
    entries.all.iter().position(|item| Rc::ptr_eq(item, of))
}

fn main() {
    let one = Rc::new(1u8);
    let other = Rc::new(1u8);
    let items = vec![Rc::clone(&other), Rc::clone(&one), Rc::clone(&one)];
    println!("{:?} {:?}", position_of(&items, &one), position_of(&items, &Rc::new(1)));
    println!("{}", Rc::ptr_eq(&one, &other));

    let square = Source::Shape(Rc::new(Shape { sides: 4 }));
    let triangle = Source::Make(Box::new(|| Rc::new(Shape { sides: 3 })));
    println!("{} {}", square.sides(), triangle.sides());
}
