//! A program's types deriving `Debug`, beside a library that writes a `dyn`
//! it's given: none of them is ever made a `dyn` nor given to the library,
//! so their writers aren't framed, and their dictionaries, which generic
//! code writes, the program's own, an `Option`'s and a `Vec`'s, need no
//! `fmtInto`. They were
//! refused, the Canvas app's `Viewport` and `Region` (#1772), and so was
//! `Region` again, held by another's field, whose derived `Debug` makes a
//! `dyn` of it that only std's builder is given (#1778).
use dep::label;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub left: f64,
    pub top: f64,
}

#[derive(Debug)]
pub struct Nested {
    pub region: Region,
}

/// Generic code, which writes what it's given by its dictionary.
fn shown<T: std::fmt::Debug>(value: &T) -> String {
    format!("<{value:?}>")
}

pub fn main() {
    let view = Viewport { x: 1.5, y: -2.0, zoom: 1.25 };
    let bounds = Some(Region { left: 0.0, top: 3.5 });
    let views = vec![view, Viewport { zoom: 2.0, ..view }];
    println!("{view:?} {bounds:?} {views:?}");
    println!("{:#?}", bounds);
    println!("{} {}", shown(&view), shown(&bounds));
    let nested = Nested { region: Region { left: 1.0, top: -1.0 } };
    println!("{nested:?} {}", shown(&nested));
    println!("{}", label());
}
